use crate::core::config::setting_builders::SchemaBuilder;
use crate::utils::{collapse_repeated_spaces, get_current_time};
use async_trait::async_trait;
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tracing::error;
use zerolaunch_plugin_api::config::{
    ComponentCore, ComponentType, ConfigError, Configurable, SettingDefinition,
};
use zerolaunch_plugin_api::{
    CachedCandidateData, CandidateId, ScoreBooster, ScoreDetail, ScoreDetailKind, ScoredCandidate,
};

/// 查询亲和度数据
#[derive(Debug, Clone, Serialize, Deserialize)]
struct QueryAffinityData {
    /// 衰减后的有效次数（浮点数，支持衰减累积）
    effective_count: f64,
    /// 最后一次启动时间（用于计算时的衰减）
    last_launch_time: i64,
    /// 最后一次记录计数的时间（用于冷却机制）
    last_record_time: i64,
}

impl QueryAffinityData {
    fn new(current_time: i64) -> Self {
        Self {
            effective_count: 1.0,
            last_launch_time: current_time,
            last_record_time: current_time,
        }
    }
}

/// 查询亲和度增强器的强类型配置结构。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryAffinitySettings {
    #[serde(
        rename = "query_affinity_weight",
        default = "default_query_affinity_weight"
    )]
    pub query_affinity_weight: f64,
    #[serde(
        rename = "query_affinity_time_decay",
        default = "default_query_affinity_time_decay"
    )]
    pub query_affinity_time_decay: f64,
    #[serde(
        rename = "query_affinity_cooldown",
        default = "default_query_affinity_cooldown"
    )]
    pub query_affinity_cooldown: f64,
}

impl Default for QueryAffinitySettings {
    fn default() -> Self {
        Self {
            query_affinity_weight: default_query_affinity_weight(),
            query_affinity_time_decay: default_query_affinity_time_decay(),
            query_affinity_cooldown: default_query_affinity_cooldown(),
        }
    }
}

fn default_query_affinity_weight() -> f64 {
    3.0
}
fn default_query_affinity_time_decay() -> f64 {
    259200.0
}
fn default_query_affinity_cooldown() -> f64 {
    15.0
}

/// 查询亲和度增强器内部实现
#[derive(Debug)]
struct QueryAffinityBoosterInner {
    /// 查询亲和度映射: method_text -> 该程序的查询-启动关联历史
    /// （按程序分桶；boost 前缀扫描只需遍历当前候选对应桶）
    method_affinity: DashMap<String, Vec<QueryAffinityEntry>>,
}

/// 单个程序的一条查询-启动关联记录
#[derive(Debug, Clone)]
struct QueryAffinityEntry {
    /// 用户启动该程序时输入的完整查询（已归一化小写 + 折叠空格）
    recorded_query: String,
    /// 衰减后的有效次数与时间信息
    data: QueryAffinityData,
}

impl QueryAffinityBoosterInner {
    fn new() -> Self {
        QueryAffinityBoosterInner {
            method_affinity: DashMap::new(),
        }
    }

    /// 记录查询-程序启动关联（衰减累积 + 冷却机制）
    /// query 须已归一化（小写 + 折叠空格），record 由调用方统一归一化。
    fn record_query_launch(&self, query: &str, method_text: &str, cooldown: i64, time_decay: i64) {
        let current_time = get_current_time();

        let mut bucket = self
            .method_affinity
            .entry(method_text.to_string())
            .or_default();
        // 同 method 下若已有同 query 记录 → 衰减累积；否则新增一条。
        if let Some(entry) = bucket.iter_mut().find(|e| e.recorded_query == query) {
            // 冷却机制：检查距离上次记录是否超过冷却时间
            let time_since_last_record = current_time - entry.data.last_record_time;
            if time_since_last_record >= cooldown {
                // 衰减累积：先对旧的有效次数进行衰减，再加上新的一次
                let time_diff = current_time - entry.data.last_launch_time;
                let decay = (-(time_diff as f64) / (time_decay as f64 + 1.0)).exp();

                entry.data.effective_count = entry.data.effective_count * decay + 1.0;
                entry.data.last_record_time = current_time;
            }
            // 无论是否在冷却时间内，都更新最后启动时间
            entry.data.last_launch_time = current_time;
        } else {
            bucket.push(QueryAffinityEntry {
                recorded_query: query.to_string(),
                data: QueryAffinityData::new(current_time),
            });
        }
    }

    /// 计算单条记录的当前有效次数（应用时间衰减）
    fn effective_count_with_decay(
        data: &QueryAffinityData,
        current_time: i64,
        time_decay: i64,
    ) -> f64 {
        let time_diff = current_time - data.last_launch_time;
        // 时间衰减因子: exp(-(时间差/时间常数))
        let decay_factor = (-(time_diff as f64) / (time_decay as f64 + 1.0)).exp();
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
    /// * `time_decay` - 时间衰减常数
    fn calculate_query_affinity_score(
        &self,
        query: &str,
        method_text: &str,
        time_decay: i64,
    ) -> f64 {
        if query.is_empty() {
            return 0.0;
        }
        let current_time = get_current_time();
        let Some(bucket) = self.method_affinity.get(method_text) else {
            return 0.0;
        };
        // 精确命中：直接取该记录的对数分（不折减）。
        if let Some(entry) = bucket.iter().find(|e| e.recorded_query == query) {
            let effective = Self::effective_count_with_decay(&entry.data, current_time, time_decay);
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
        let effective = Self::effective_count_with_decay(&entry.data, current_time, time_decay);
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
            .group(t_key!("query-affinity-booster", "groups.weight"))
            .order(0)
            .default(3.0)
            .min(0.0)
            .max(20.0)
            .step(0.1)
            .build(),
            SchemaBuilder::number(
                "query_affinity_time_decay",
                t_key!(
                    "query-affinity-booster",
                    "fields.query_affinity_time_decay.label"
                ),
                t_key!(
                    "query-affinity-booster",
                    "fields.query_affinity_time_decay.desc"
                ),
            )
            .group(t_key!("query-affinity-booster", "groups.decay"))
            .order(1)
            .default(259200.0)
            .min(60.0)
            .max(2592000.0)
            .step(60.0)
            .build(),
            SchemaBuilder::number(
                "query_affinity_cooldown",
                t_key!(
                    "query-affinity-booster",
                    "fields.query_affinity_cooldown.label"
                ),
                t_key!(
                    "query-affinity-booster",
                    "fields.query_affinity_cooldown.desc"
                ),
            )
            .group(t_key!("query-affinity-booster", "groups.cooldown"))
            .order(2)
            .default(15.0)
            .min(1.0)
            .max(300.0)
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
                settings.query_affinity_cooldown as i64,
                settings.query_affinity_time_decay as i64,
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
                settings.query_affinity_time_decay as i64,
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
    ///（直接插记录，绕开时间衰减，聚焦前缀/精确匹配与折减曲线逻辑）。
    fn inner_with_records(records: &[(&str, &str)]) -> QueryAffinityBoosterInner {
        let inner = QueryAffinityBoosterInner::new();
        // 用超大衰减常数近似免衰减（测试内时间差极小，衰减≈1）
        let time_decay = 3_600_000i64;
        let cooldown = 0i64;
        for (query, method) in records {
            inner.record_query_launch(query, method, cooldown, time_decay);
        }
        inner
    }

    /// 归一化是调用方职责（record/boost 已统一），inner 直接收归一化串。
    const METHOD_A: &str = "app-a.exe";

    #[test]
    fn exact_match_gives_full_log_score() {
        let inner = inner_with_records(&[("visual studio code", METHOD_A)]);
        // 单次有效记录 → ln(1+1)*10 ≈ 6.93
        let score = inner.calculate_query_affinity_score("visual studio code", METHOD_A, 3_600_000);
        assert!(
            (score - 1.0f64.ln_1p() * 10.0).abs() < 1e-6,
            "精确命中应得满对数分: {score}"
        );
        // 其他 method 不受影响
        assert_eq!(
            inner.calculate_query_affinity_score("visual studio code", "other.exe", 3_600_000),
            0.0
        );
    }

    #[test]
    fn prefix_match_gives_saturated_subscore() {
        let inner = inner_with_records(&[("visual studio code", METHOD_A)]);
        let full = inner.calculate_query_affinity_score("visual studio code", METHOD_A, 3_600_000);
        // 前缀 "visual st"（9/18 字符，ratio=0.5）得 0<分<满分的折减
        let prefix = inner.calculate_query_affinity_score("visual st", METHOD_A, 3_600_000);
        assert!(
            prefix > 0.0 && prefix < full,
            "前缀分应在 (0, full): {prefix} vs {full}"
        );
        // 单调性：更长前缀（更接近全长）得分更高
        let longer = inner.calculate_query_affinity_score("visual studio", METHOD_A, 3_600_000);
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
        let score = inner.calculate_query_affinity_score("vis", METHOD_A, 3_600_000);
        assert!(score > 0.0, "多前缀命中不应落空: {score}");
        // 用更长前缀验证取最长语义：输入 "visual s" 只命中长记录（短记录 "visual"
        // 不含 "visual s" 前缀），得长记录折减分；若实现错取短记录则此处为 0。
        let long_only = inner.calculate_query_affinity_score("visual s", METHOD_A, 3_600_000);
        assert!(long_only > 0.0, "应命中长记录前缀: {long_only}");
        // 输入 "visual" 精确命中短记录：满分短记录（ln2*10），不受长记录稀释。
        let exact_short = inner.calculate_query_affinity_score("visual", METHOD_A, 3_600_000);
        assert!((exact_short - 1.0f64.ln_1p() * 10.0).abs() < 1e-6);
    }

    #[test]
    fn single_char_input_gets_no_prefix_score() {
        let inner = inner_with_records(&[("visual studio code", METHOD_A)]);
        assert_eq!(
            inner.calculate_query_affinity_score("v", METHOD_A, 3_600_000),
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
            inner.calculate_query_affinity_score("Visual  Studio Code", METHOD_A, 3_600_000),
            0.0
        );
        assert_eq!(
            inner.calculate_query_affinity_score("visual  studio", METHOD_A, 3_600_000),
            0.0
        );
    }
}
