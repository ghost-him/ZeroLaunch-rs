use crate::core::config::setting_builders::SchemaBuilder;
use crate::utils::collapse_repeated_spaces;
use async_trait::async_trait;
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::{error, warn};
use zerolaunch_plugin_api::config::{
    ComponentCore, ComponentType, ConfigError, Configurable, SettingDefinition,
};
use zerolaunch_plugin_api::{
    CachedCandidateData, CandidateId, ScoreBooster, ScoreDetail, ScoreDetailKind, ScoredCandidate,
};

/// 每个目标保留的查询关联记录条数上限，超出时淘汰最久未启动的一条。
/// 约束运行态文件规模：查询串种类随使用时间增长，不设上限将无界膨胀。
const MAX_AFFINITY_ENTRIES_PER_METHOD: usize = 128;

/// 查询亲和度数据 —— 单条查询-启动关联的衰减计数与启动序号锚点。
///
/// 仅限本文件内使用，作为 `QueryAffinityEntry` 的字段。
#[derive(Debug, Clone, Serialize, Deserialize)]
struct QueryAffinityData {
    /// 衰减后的有效启动次数（浮点数，支持按启动事件衰减累积）。
    #[serde(
        rename = "effective_count",
        default = "default_affinity_effective_count"
    )]
    effective_count: f64,
    /// 最后一次记录该关联时的全局启动序号，用于按启动事件计算衰减距离。
    #[serde(rename = "last_seq", default = "default_affinity_seq")]
    last_seq: u64,
}

/// 关联记录缺失有效次数时的默认值：按一次启动计。
fn default_affinity_effective_count() -> f64 {
    1.0
}

/// 关联记录缺失启动序号时的默认值：0，表示尚未按新事件衰减。
fn default_affinity_seq() -> u64 {
    0
}

impl QueryAffinityData {
    fn new(last_seq: u64) -> Self {
        Self {
            effective_count: 1.0,
            last_seq,
        }
    }
}

/// 查询亲和度增强器的强类型配置结构。
///
/// 反序列化自配置文件中 "query-affinity-booster" 组件的 settings 段。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryAffinitySettings {
    /// 查询亲和分的权重；越大影响越强，0 表示关闭。
    #[serde(
        rename = "query_affinity_weight",
        default = "default_query_affinity_weight"
    )]
    pub query_affinity_weight: f64,
    /// 事件衰减常数（启动次数）；越大衰减越慢。
    #[serde(
        rename = "query_affinity_event_decay",
        default = "default_query_affinity_event_decay"
    )]
    pub query_affinity_event_decay: f64,
}

impl Default for QueryAffinitySettings {
    fn default() -> Self {
        Self {
            query_affinity_weight: default_query_affinity_weight(),
            query_affinity_event_decay: default_query_affinity_event_decay(),
        }
    }
}

fn default_query_affinity_weight() -> f64 {
    3.0
}
fn default_query_affinity_event_decay() -> f64 {
    500.0
}

/// 查询亲和度增强器的持久化运行态快照。
///
/// 由 `QueryAffinityBoosterInner::snapshot` 产出、`QueryAffinityBoosterInner::restore`
/// 消费，经 ConfigManager 写入独立的运行态文件（与用户配置分离）。仅限本文件内使用。
#[derive(Debug, Clone, Serialize, Deserialize)]
struct QueryAffinityState {
    /// 各目标的查询关联记录：目标标识 → 记录列表（每项为该程序的一条查询-启动关联）
    #[serde(rename = "method_affinity", default)]
    method_affinity: HashMap<String, Vec<QueryAffinityEntry>>,
    /// 全局启动事件序号，用于跨重启保持衰减距离连续。
    #[serde(rename = "seq", default = "default_affinity_seq")]
    seq: u64,
}

/// 查询亲和度增强器内部实现。
///
/// 仅限本文件内使用，由 `QueryAffinityBooster` 持有。
#[derive(Debug)]
struct QueryAffinityBoosterInner {
    /// 查询亲和度映射: method_text -> 该程序的查询-启动关联历史
    /// （按程序分桶；boost 前缀扫描只需遍历当前候选对应桶）
    method_affinity: DashMap<String, Vec<QueryAffinityEntry>>,
    /// 全局启动事件序号：每次记录关联自增，作为衰减时钟。
    seq: AtomicU64,
}

/// 单个程序的一条查询-启动关联记录
#[derive(Debug, Clone, Serialize, Deserialize)]
struct QueryAffinityEntry {
    /// 用户启动该程序时输入的完整查询（已归一化小写 + 折叠空格）
    #[serde(rename = "recorded_query", default)]
    recorded_query: String,
    /// 衰减计数与最近一次记录的启动序号
    #[serde(rename = "data", default)]
    data: QueryAffinityData,
}

impl Default for QueryAffinityData {
    fn default() -> Self {
        Self {
            effective_count: default_affinity_effective_count(),
            last_seq: default_affinity_seq(),
        }
    }
}

impl QueryAffinityBoosterInner {
    fn new() -> Self {
        QueryAffinityBoosterInner {
            method_affinity: DashMap::new(),
            seq: AtomicU64::new(0),
        }
    }

    /// 记录查询-程序启动关联（按全局启动序号衰减累积）。
    /// query 须已归一化（小写 + 折叠空格），record 由调用方统一归一化。
    fn record_query_launch(&self, query: &str, method_text: &str, event_decay: f64) {
        let current_seq = self.seq.fetch_add(1, Ordering::Relaxed) + 1;

        let mut bucket = self
            .method_affinity
            .entry(method_text.to_string())
            .or_default();
        // 同 method 下若已有同 query 记录 → 按事件距离衰减累积；否则新增一条。
        if let Some(entry) = bucket.iter_mut().find(|e| e.recorded_query == query) {
            let distance = current_seq.saturating_sub(entry.data.last_seq);
            let decay = (-(distance as f64) / event_decay).exp();

            entry.data.effective_count = entry.data.effective_count * decay + 1.0;
            entry.data.last_seq = current_seq;
        } else {
            bucket.push(QueryAffinityEntry {
                recorded_query: query.to_string(),
                data: QueryAffinityData::new(current_seq),
            });
            // 条目上限：淘汰最久未记录的一条（序号并列时保留先插入者，不会淘汰刚插入的记录）
            if bucket.len() > MAX_AFFINITY_ENTRIES_PER_METHOD {
                let oldest = bucket
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, entry)| entry.data.last_seq)
                    .map(|(index, _)| index);
                if let Some(index) = oldest {
                    bucket.remove(index);
                }
            }
        }
    }

    /// 计算单条记录的当前有效次数（按启动事件距离衰减）
    fn effective_count_with_decay(
        data: &QueryAffinityData,
        current_seq: u64,
        event_decay: f64,
    ) -> f64 {
        let distance = current_seq.saturating_sub(data.last_seq);
        // 事件衰减因子: exp(-(启动事件距离/衰减常数))
        let decay_factor = (-(distance as f64) / event_decay).exp();
        data.effective_count * decay_factor
    }

    /// 将当前有效次数映射为对数缩放分（与历史分同量级，避免分数过大）
    fn affinity_log_score(effective_count: f64) -> f64 {
        effective_count.ln_1p() * 10.0
    }

    /// 计算查询亲和分数：精确命中或最长前缀命中的感知曲线折减分
    ///
    /// 语义：只有用户曾以完整查询启动过该程序，才对它的前缀输入产生亲和。
    /// 前缀命中取记录中最长的一条（最贴近当前输入的完整历史），
    /// 其原始对数分乘以饱和感知曲线 sat(ratio)，ratio = 输入长度/该完整查询长度。
    /// 短前缀（低 ratio）被曲线强压制，接近全长时趋近完整命中分。
    /// 输入长度 < 2 视为无前缀意图，不给前缀分（避免单字符干扰首字符排序）。
    ///
    /// # Arguments
    /// * `query` - 当前输入（已归一化）
    /// * `method_text` - 候选程序标识
    /// * `event_decay` - 启动事件衰减常数（次数）
    fn calculate_query_affinity_score(
        &self,
        query: &str,
        method_text: &str,
        event_decay: f64,
    ) -> f64 {
        if query.is_empty() {
            return 0.0;
        }
        let current_seq = self.seq.load(Ordering::Relaxed);
        let Some(bucket) = self.method_affinity.get(method_text) else {
            return 0.0;
        };
        // 精确命中：直接取该记录的对数分（不折减）。
        if let Some(entry) = bucket.iter().find(|e| e.recorded_query == query) {
            let effective = Self::effective_count_with_decay(&entry.data, current_seq, event_decay);
            return Self::affinity_log_score(effective);
        }
        // 前缀命中：输入长度须 ≥ 2 才算前缀意图。
        if query.chars().count() < 2 {
            return 0.0;
        }
        // 找 recorded_query 以 query 开头的最长一条（精确相等已在上方分支返回，
        // 此处 strip_prefix 命中即 recorded_query 严格长于 query）。
        let mut best: Option<(&QueryAffinityEntry, f64)> = None; // (entry, ratio)
        for entry in bucket.iter() {
            if entry.recorded_query.strip_prefix(query).is_none() {
                continue;
            }
            let recorded_len = entry.recorded_query.chars().count() as f64;
            let ratio = query.chars().count() as f64 / recorded_len;
            let is_longer = best
                .as_ref()
                .map(|(be, _)| {
                    entry.recorded_query.chars().count() > be.recorded_query.chars().count()
                })
                .unwrap_or(true);
            if is_longer {
                best = Some((entry, ratio));
            }
        }
        let Some((entry, ratio)) = best else {
            return 0.0;
        };
        let effective = Self::effective_count_with_decay(&entry.data, current_seq, event_decay);
        let log_score = Self::affinity_log_score(effective);
        log_score * Self::prefix_saturation(ratio)
    }

    /// 前缀感知饱和曲线：短前缀强压制，接近全长时趋近 1
    ///
    /// 形状：1 - exp(-k·ratio)（k=2.2），在 ratio∈(0,1] 单调增、下凸。
    /// 标定依据（见设计分析）：单字符(ratio≈0.1)≈0.19，半程(0.5)≈0.67，
    /// 3/4(0.75)≈0.81，全长(1.0)≈0.89。短前缀不会被历史长查询带上高分，
    /// 杜绝「输入 qq 时 QQ 与 QQ音乐 平起平坐」的原始问题。
    fn prefix_saturation(ratio: f64) -> f64 {
        let k = 2.2;
        1.0 - (-k * ratio).exp()
    }

    /// 导出运行态快照（供 ConfigManager 写入运行态文件）
    fn snapshot(&self) -> QueryAffinityState {
        QueryAffinityState {
            method_affinity: self
                .method_affinity
                .iter()
                .map(|e| (e.key().clone(), e.value().clone()))
                .collect(),
            seq: self.seq.load(Ordering::Relaxed),
        }
    }

    /// 从运行态快照重建内部状态
    fn restore(state: QueryAffinityState) -> Self {
        QueryAffinityBoosterInner {
            method_affinity: state.method_affinity.into_iter().collect(),
            seq: AtomicU64::new(state.seq),
        }
    }
}

/// 查询亲和度增强器 - 基于查询词与候选项的关联关系对候选项进行分数增强
#[derive(Debug)]
pub struct QueryAffinityBooster {
    core: ComponentCore,
    inner: RwLock<QueryAffinityBoosterInner>,
    settings: RwLock<QueryAffinitySettings>,
}

impl Default for QueryAffinityBooster {
    fn default() -> Self {
        Self::new()
    }
}

impl QueryAffinityBooster {
    pub fn new() -> Self {
        QueryAffinityBooster {
            core: ComponentCore::new(
                "query-affinity-booster".to_string(),
                t_key!("query-affinity-booster", "name").to_string(),
                t_key!("query-affinity-booster", "description").to_string(),
                ComponentType::ScoreBooster,
                10,
            ),
            inner: RwLock::new(QueryAffinityBoosterInner::new()),
            settings: RwLock::new(QueryAffinitySettings::default()),
        }
    }
}

#[async_trait]
impl Configurable for QueryAffinityBooster {
    fn core(&self) -> &ComponentCore {
        &self.core
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![
            SchemaBuilder::number(
                "query_affinity_weight",
                t_key!(
                    "query-affinity-booster",
                    "fields.query_affinity_weight.label"
                ),
                t_key!(
                    "query-affinity-booster",
                    "fields.query_affinity_weight.desc"
                ),
            )
            .note(t_key!(
                "query-affinity-booster",
                "fields.query_affinity_weight.note"
            ))
            .group(t_key!("query-affinity-booster", "groups.weight"))
            .order(0)
            .default(3.0)
            .min(0.0)
            .max(20.0)
            .step(0.1)
            .build(),
            SchemaBuilder::number(
                "query_affinity_event_decay",
                t_key!(
                    "query-affinity-booster",
                    "fields.query_affinity_event_decay.label"
                ),
                t_key!(
                    "query-affinity-booster",
                    "fields.query_affinity_event_decay.desc"
                ),
            )
            .group(t_key!("query-affinity-booster", "groups.decay"))
            .order(1)
            .default(500.0)
            .min(1.0)
            .max(100000.0)
            .step(1.0)
            .build(),
        ]
    }

    fn get_settings(&self) -> serde_json::Value {
        serde_json::to_value(self.settings.read().clone()).unwrap_or_default()
    }

    async fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        let parsed: QueryAffinitySettings = serde_json::from_value(settings).unwrap_or_default();
        *self.settings.write() = parsed;
        Ok(())
    }

    /// 导出查询亲和运行态（各目标的查询-启动关联历史）
    fn runtime_state(&self) -> Option<serde_json::Value> {
        serde_json::to_value(self.inner.read().snapshot()).ok()
    }

    /// 恢复查询亲和运行态；运行态文件损坏时告警并从空数据开始（不影响配置项）
    fn restore_runtime_state(&self, state: serde_json::Value) {
        match serde_json::from_value::<QueryAffinityState>(state) {
            Ok(snapshot) => *self.inner.write() = QueryAffinityBoosterInner::restore(snapshot),
            Err(e) => warn!(
                "[QueryAffinityBooster] 运行态恢复失败，使用空亲和数据: {}",
                e
            ),
        }
    }
}

#[async_trait]
impl ScoreBooster for QueryAffinityBooster {
    /// 记录查询-候选项启动关联（query 归一化后入库）
    async fn record(&self, candidate_id: CandidateId, data: &CachedCandidateData, query: &str) {
        if query.trim().is_empty() {
            return;
        }
        // 归一化对齐 boost 侧：小写 + 折叠连续空格（record 收到的是用户原始输入，
        // boost 收到的是已折叠 search_term——统一两侧 key 形态才能命中）。
        let normalized = collapse_repeated_spaces(&query.to_lowercase());
        if normalized.is_empty() {
            return;
        }
        if let Some(search_candidate) = data.get_candidate(candidate_id) {
            let method_text = search_candidate.target.payload();
            let settings = self.settings.read();
            self.inner.read().record_query_launch(
                &normalized,
                method_text,
                settings.query_affinity_event_decay,
            );
        } else {
            error!(
                "[QueryAffinityBooster] 无法找到候选项数据，无法记录查询启动关联，candidate_id: {}",
                candidate_id
            );
        }
    }

    /// 基于查询亲和度增强候选项分数
    async fn boost(
        &self,
        candidates: &mut Vec<ScoredCandidate>,
        data: &CachedCandidateData,
        query: &str,
    ) {
        if query.trim().is_empty() {
            return;
        }
        // 归一化对齐 record 侧（见 record 注释）。
        let normalized = collapse_repeated_spaces(&query.to_lowercase());
        if normalized.is_empty() {
            return;
        }

        let inner = self.inner.read();
        let settings = self.settings.read();

        for candidate in candidates.iter_mut() {
            let method_text = match data.get_candidate(candidate.candidate_id) {
                Some(sc) => sc.target.payload(),
                None => continue,
            };

            let affinity_score = inner.calculate_query_affinity_score(
                &normalized,
                method_text,
                settings.query_affinity_event_decay,
            );
            let boost_value = settings.query_affinity_weight * affinity_score;
            candidate.score += boost_value;

            candidate.detailed_score.push(ScoreDetail {
                score: affinity_score,
                weight: settings.query_affinity_weight,
                description: "查询亲和分数".to_string(),
                kind: ScoreDetailKind::Add,
            });
        }
    }
}

use crate::plugin_framework::builtin_registry::ScoreBoosterEntry;
use std::sync::Arc;

pub(crate) fn build_query_affinity_booster() -> (Arc<dyn Configurable>, Arc<dyn ScoreBooster>) {
    let booster: Arc<dyn ScoreBooster> = Arc::new(QueryAffinityBooster::new());
    let configurable: Arc<dyn Configurable> = booster.clone();
    (configurable, booster)
}

::inventory::submit! {
    ScoreBoosterEntry {
        component_id: "query-affinity-booster",
        priority: 10,
        factory: build_query_affinity_booster,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造 inner，向指定 method 记录若干条「完整查询 → 启动」关联
    ///（直接插记录，使用极大衰减常数近似免衰减，聚焦前缀/精确匹配与折减曲线逻辑）。
    fn inner_with_records(records: &[(&str, &str)]) -> QueryAffinityBoosterInner {
        let inner = QueryAffinityBoosterInner::new();
        let event_decay = 1e12;
        for (query, method) in records {
            inner.record_query_launch(query, method, event_decay);
        }
        inner
    }

    /// 归一化是调用方职责（record/boost 已统一），inner 直接收归一化串。
    const METHOD_A: &str = "app-a.exe";

    #[test]
    fn exact_match_gives_full_log_score() {
        let inner = inner_with_records(&[("visual studio code", METHOD_A)]);
        // 单次有效记录 → ln(1+1)*10 ≈ 6.93
        let score = inner.calculate_query_affinity_score("visual studio code", METHOD_A, 1e12);
        assert!(
            (score - 1.0f64.ln_1p() * 10.0).abs() < 1e-6,
            "精确命中应得满对数分: {score}"
        );
        // 其他 method 不受影响
        assert_eq!(
            inner.calculate_query_affinity_score("visual studio code", "other.exe", 1e12),
            0.0
        );
    }

    #[test]
    fn decay_follows_launch_events_not_wall_clock() {
        let inner = QueryAffinityBoosterInner::new();
        inner.record_query_launch("wps", METHOD_A, 100.0);
        let full = inner.calculate_query_affinity_score("wps", METHOD_A, 100.0);

        // 用无关启动事件推进全局序号，wps 关联应随事件距离衰减。
        for _ in 0..100 {
            inner.record_query_launch("other", "other.exe", 100.0);
        }
        let decayed = inner.calculate_query_affinity_score("wps", METHOD_A, 100.0);

        assert!(decayed < full, "事件推进后应衰减: {decayed} vs {full}");
        assert!(decayed > 0.0, "衰减后仍应保留部分亲和: {decayed}");
    }

    #[test]
    fn prefix_match_gives_saturated_subscore() {
        let inner = inner_with_records(&[("visual studio code", METHOD_A)]);
        let full = inner.calculate_query_affinity_score("visual studio code", METHOD_A, 1e12);
        // 前缀 "visual st"（9/18 字符，ratio=0.5）得 0<分<满分的折减
        let prefix = inner.calculate_query_affinity_score("visual st", METHOD_A, 1e12);
        assert!(
            prefix > 0.0 && prefix < full,
            "前缀分应在 (0, full): {prefix} vs {full}"
        );
        // 单调性：更长前缀（更接近全长）得分更高
        let longer = inner.calculate_query_affinity_score("visual studio", METHOD_A, 1e12);
        assert!(longer > prefix, "更长前缀应更高: {longer} vs {prefix}");
        assert!(longer < full, "前缀分仍应小于精确分: {longer} vs {full}");
    }

    #[test]
    fn prefix_score_respects_saturation_curve_shape() {
        // 直接验证饱和曲线形状（纯函数）：单调增、下凸、值域 (0,1)
        let s = |ratio: f64| QueryAffinityBoosterInner::prefix_saturation(ratio);
        assert!(
            s(0.1) > 0.0 && s(0.1) < 0.25,
            "极短前缀应被强压制: {}",
            s(0.1)
        );
        assert!(s(0.5) > 0.5, "半程应过半: {}", s(0.5));
        assert!(s(1.0) > 0.85, "全长应接近 1: {}", s(1.0));
        assert!(s(0.2) < s(0.5) && s(0.5) < s(0.9), "单调增");
        // 下凸：中点折减不如线性（差 = 中 - 两端平均 > 0 → sat 凸）
        let convexity = s(0.5) - (s(0.1) + s(0.9)) / 2.0;
        assert!(
            convexity > 0.0,
            "曲线应下凸(短前缀压制): convexity={convexity}"
        );
    }

    #[test]
    fn multi_prefix_takes_longest_record() {
        let inner = inner_with_records(&[("visual", METHOD_A), ("visual studio code", METHOD_A)]);
        // 输入 "vis" 命中两条前缀记录（visual / visual studio code），
        // 应取最长记录 "visual studio code" 作折减基准（ratio=3/18）。
        let score = inner.calculate_query_affinity_score("vis", METHOD_A, 1e12);
        assert!(score > 0.0, "多前缀命中不应落空: {score}");
        // 用更长前缀验证取最长语义：输入 "visual s" 只命中长记录（短记录 "visual"
        // 不含 "visual s" 前缀），得长记录折减分；若实现错取短记录则此处为 0。
        let long_only = inner.calculate_query_affinity_score("visual s", METHOD_A, 1e12);
        assert!(long_only > 0.0, "应命中长记录前缀: {long_only}");
        // 输入 "visual" 精确命中短记录：满分短记录（ln2*10），不受长记录稀释。
        let exact_short = inner.calculate_query_affinity_score("visual", METHOD_A, 1e12);
        assert!((exact_short - 1.0f64.ln_1p() * 10.0).abs() < 1e-6);
    }

    #[test]
    fn single_char_input_gets_no_prefix_score() {
        let inner = inner_with_records(&[("visual studio code", METHOD_A)]);
        assert_eq!(
            inner.calculate_query_affinity_score("v", METHOD_A, 1e12),
            0.0,
            "单字符不算前缀意图"
        );
    }

    #[test]
    fn unnormalized_query_does_not_match() {
        // inner 契约：只收已归一化串。若调用方漏归一化，大写/多余空格不应命中
        //（防回归：record/boost 两侧必须统一归一化）
        let inner = inner_with_records(&[("visual studio code", METHOD_A)]);
        assert_eq!(
            inner.calculate_query_affinity_score("Visual  Studio Code", METHOD_A, 1e12),
            0.0
        );
        assert_eq!(
            inner.calculate_query_affinity_score("visual  studio", METHOD_A, 1e12),
            0.0
        );
    }
}
