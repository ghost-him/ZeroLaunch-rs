use crate::core::config::setting_builders::SchemaBuilder;
use crate::utils::{days_between, generate_current_date, get_current_time};
use async_trait::async_trait;
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use tracing::{error, warn};
use zerolaunch_plugin_api::config::{
    ComponentCore, ComponentType, ConfigError, Configurable, SettingDefinition,
};
use zerolaunch_plugin_api::{
    CachedCandidateData, CandidateId, ScoreBooster, ScoreDetail, ScoreDetailKind, ScoredCandidate,
};

/// 历史记录增强器的强类型配置结构。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryBoosterSettings {
    #[serde(rename = "history_weight", default = "default_history_weight")]
    pub history_weight: f64,
    #[serde(
        rename = "recent_habit_weight",
        default = "default_recent_habit_weight"
    )]
    pub recent_habit_weight: f64,
    #[serde(rename = "temporal_weight", default = "default_temporal_weight")]
    pub temporal_weight: f64,
    #[serde(rename = "temporal_decay", default = "default_temporal_decay")]
    pub temporal_decay: f64,
}

impl Default for HistoryBoosterSettings {
    fn default() -> Self {
        Self {
            history_weight: default_history_weight(),
            recent_habit_weight: default_recent_habit_weight(),
            temporal_weight: default_temporal_weight(),
            temporal_decay: default_temporal_decay(),
        }
    }
}

fn default_history_weight() -> f64 {
    0.8
}
fn default_recent_habit_weight() -> f64 {
    1.5
}
fn default_temporal_weight() -> f64 {
    0.5
}
fn default_temporal_decay() -> f64 {
    10800.0
}

/// 历史记录增强器的持久化运行态快照。
///
/// 由 `HistoryBoosterInner::snapshot` 产出、`HistoryBoosterInner::restore` 消费，
/// 经 ConfigManager 写入独立的运行态文件（与用户配置分离）。仅限本文件内使用。
#[derive(Debug, Clone, Serialize, Deserialize)]
struct HistoryBoosterState {
    /// 最近 7 天启动次数桶，由新到旧排列
    #[serde(rename = "launch_time", default)]
    launch_time: Vec<LaunchBucketState>,
    /// 历史总启动次数：目标标识 → 累计启动次数
    #[serde(rename = "history_launch_time", default)]
    history_launch_time: HashMap<String, u64>,
    /// 最近一次启动时间：目标标识 → unix 秒时间戳
    #[serde(rename = "latest_launch_time", default)]
    latest_launch_time: HashMap<String, i64>,
}

/// 单个日期桶的运行态快照（该日各目标的启动次数）。
/// 仅限本文件内使用，作为 `HistoryBoosterState::launch_time` 的元素。
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LaunchBucketState {
    /// 桶所属日期（`%Y-%m-%d`，本地时区）
    #[serde(rename = "date", default)]
    date: String,
    /// 该日各目标的启动次数：目标标识 → 次数
    #[serde(rename = "counts", default)]
    counts: HashMap<String, u64>,
}

/// 历史记录增强器内部实现
#[derive(Debug)]
struct HistoryBoosterInner {
    /// 最近 7 天的启动次数桶：由新到旧，每桶携带所属日期
    launch_time: VecDeque<(String, DashMap<String, u64>)>,
    /// 历史总启动次数
    history_launch_time: DashMap<String, u64>,
    /// 最近一次启动时间（时间戳）
    latest_launch_time: DashMap<String, i64>,
}

impl HistoryBoosterInner {
    fn new() -> Self {
        let mut launch_time = VecDeque::new();
        launch_time.push_front((generate_current_date(), DashMap::new()));
        HistoryBoosterInner {
            launch_time,
            history_launch_time: DashMap::new(),
            latest_launch_time: DashMap::new(),
        }
    }

    /// 记录程序启动，更新所有统计数据
    fn record_launch(&mut self, method_text: &str) {
        // 确保日期信息已更新
        self.update_launch_info();

        let method_key = method_text.to_string();
        let current_time = get_current_time();

        // 更新今日启动次数（front 桶恒为今天，见 update_launch_info）
        self.launch_time[0]
            .1
            .entry(method_key.clone())
            .and_modify(|count| *count += 1)
            .or_insert(1);

        // 更新历史总启动次数
        self.history_launch_time
            .entry(method_key.clone())
            .and_modify(|count| *count += 1)
            .or_insert(1);

        // 更新最近启动时间
        self.latest_launch_time.insert(method_key, current_time);
    }

    /// 计算历史总分（基于所有历史启动次数，对数缩放）
    fn calculate_history_score(&self, method_text: &str) -> f64 {
        if let Some(count) = self.history_launch_time.get(method_text) {
            (*count as f64).ln_1p()
        } else {
            0.0
        }
    }

    /// 计算近期习惯分数（基于最近7天的启动次数，带衰减）
    fn calculate_recent_habit_score(&self, method_text: &str) -> f64 {
        let mut result: f64 = 0.0;
        let mut k: f64 = 1.0;
        self.launch_time.iter().for_each(|(_, day)| {
            if let Some(time) = day.get(method_text) {
                result += (*time as f64) * k;
            }
            k /= 1.3
        });
        result
    }

    /// 计算近期热度分数
    fn calculate_temporal_score(&self, method_text: &str, temporal_decay: i64) -> f64 {
        if let Some(last_launch_time) = self.latest_launch_time.get(method_text) {
            let current_time = get_current_time();
            let time_diff = current_time - *last_launch_time;
            let k = 6.0;
            k / (1.0 + (time_diff as f64) / (temporal_decay as f64 + 1.0))
        } else {
            0.0
        }
    }

    /// 将 7 天桶对齐到今天：按实际天数补齐中间空桶，跨度 ≥7 天时旧桶全部作废。
    /// 调用方：记录启动前、恢复运行态后。
    fn update_launch_info(&mut self) {
        let today = generate_current_date();
        if self.launch_time.front().map(|(date, _)| date.as_str()) == Some(today.as_str()) {
            return;
        }

        match self
            .launch_time
            .front()
            .and_then(|(date, _)| days_between(date, &today))
        {
            // 与今天相差 1..7 天：逐日补齐空桶
            Some(days) if days < 7 => {
                for _ in 0..days {
                    self.launch_time.push_front((today.clone(), DashMap::new()));
                }
            }
            // 跨度 ≥7 天 / 日期无法解析 / 时钟回拨：旧桶全部过期，以今天重建
            _ => {
                self.launch_time.clear();
                self.launch_time.push_front((today, DashMap::new()));
            }
        }

        while self.launch_time.len() > 7 {
            self.launch_time.pop_back();
        }
    }

    /// 导出运行态快照（供 ConfigManager 写入运行态文件）
    fn snapshot(&self) -> HistoryBoosterState {
        HistoryBoosterState {
            launch_time: self
                .launch_time
                .iter()
                .map(|(date, counts)| LaunchBucketState {
                    date: date.clone(),
                    counts: counts
                        .iter()
                        .map(|e| (e.key().clone(), *e.value()))
                        .collect(),
                })
                .collect(),
            history_launch_time: self
                .history_launch_time
                .iter()
                .map(|e| (e.key().clone(), *e.value()))
                .collect(),
            latest_launch_time: self
                .latest_launch_time
                .iter()
                .map(|e| (e.key().clone(), *e.value()))
                .collect(),
        }
    }

    /// 从运行态快照重建内部状态（日期桶对齐由调用方随后执行）
    fn restore(state: HistoryBoosterState) -> Self {
        let launch_time = state
            .launch_time
            .into_iter()
            .map(|bucket| (bucket.date, bucket.counts.into_iter().collect()))
            .collect();
        HistoryBoosterInner {
            launch_time,
            history_launch_time: state.history_launch_time.into_iter().collect(),
            latest_launch_time: state.latest_launch_time.into_iter().collect(),
        }
    }
}

/// 历史记录增强器 - 基于历史启动次数、近期习惯和热度对候选项进行分数增强
#[derive(Debug)]
pub struct HistoryBooster {
    core: ComponentCore,
    inner: RwLock<HistoryBoosterInner>,
    settings: RwLock<HistoryBoosterSettings>,
}

impl Default for HistoryBooster {
    fn default() -> Self {
        Self::new()
    }
}

impl HistoryBooster {
    pub fn new() -> Self {
        HistoryBooster {
            core: ComponentCore::new(
                "history-booster".to_string(),
                t_key!("history-booster", "name").to_string(),
                t_key!("history-booster", "description").to_string(),
                ComponentType::ScoreBooster,
                0,
            ),
            inner: RwLock::new(HistoryBoosterInner::new()),
            settings: RwLock::new(HistoryBoosterSettings::default()),
        }
    }
}

#[async_trait]
impl Configurable for HistoryBooster {
    fn core(&self) -> &ComponentCore {
        &self.core
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![
            SchemaBuilder::number(
                "history_weight",
                t_key!("history-booster", "fields.history_weight.label"),
                t_key!("history-booster", "fields.history_weight.desc"),
            )
            .group(t_key!("history-booster", "groups.weight"))
            .order(0)
            .default(0.8)
            .min(0.0)
            .max(10.0)
            .step(0.1)
            .build(),
            SchemaBuilder::number(
                "recent_habit_weight",
                t_key!("history-booster", "fields.recent_habit_weight.label"),
                t_key!("history-booster", "fields.recent_habit_weight.desc"),
            )
            .group(t_key!("history-booster", "groups.weight"))
            .order(1)
            .default(1.5)
            .min(0.0)
            .max(10.0)
            .step(0.1)
            .build(),
            SchemaBuilder::number(
                "temporal_weight",
                t_key!("history-booster", "fields.temporal_weight.label"),
                t_key!("history-booster", "fields.temporal_weight.desc"),
            )
            .group(t_key!("history-booster", "groups.weight"))
            .order(2)
            .default(0.5)
            .min(0.0)
            .max(10.0)
            .step(0.1)
            .build(),
            SchemaBuilder::number(
                "temporal_decay",
                t_key!("history-booster", "fields.temporal_decay.label"),
                t_key!("history-booster", "fields.temporal_decay.desc"),
            )
            .group(t_key!("history-booster", "groups.decay"))
            .order(3)
            .default(10800.0)
            .min(60.0)
            .max(86400.0)
            .step(60.0)
            .build(),
        ]
    }

    fn get_settings(&self) -> serde_json::Value {
        serde_json::to_value(self.settings.read().clone()).unwrap_or_default()
    }

    async fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        let parsed: HistoryBoosterSettings = serde_json::from_value(settings).unwrap_or_default();
        *self.settings.write() = parsed;
        Ok(())
    }

    /// 导出启动历史运行态（启动次数桶、总次数、最近启动时间）
    fn runtime_state(&self) -> Option<serde_json::Value> {
        serde_json::to_value(self.inner.read().snapshot()).ok()
    }

    /// 恢复启动历史运行态，并按当前日期对齐 7 天桶。
    /// 运行态文件损坏时告警并从空历史开始（不影响配置项）。
    fn restore_runtime_state(&self, state: serde_json::Value) {
        match serde_json::from_value::<HistoryBoosterState>(state) {
            Ok(snapshot) => {
                let mut inner = self.inner.write();
                *inner = HistoryBoosterInner::restore(snapshot);
                inner.update_launch_info();
            }
            Err(e) => warn!("[HistoryBooster] 运行态恢复失败，使用空历史: {}", e),
        }
    }
}

#[async_trait]
impl ScoreBooster for HistoryBooster {
    /// 记录候选项被选中启动
    async fn record(&self, candidate_id: CandidateId, data: &CachedCandidateData, _query: &str) {
        if let Some(search_candidate) = data.get_candidate(candidate_id) {
            let method_text = search_candidate.target.payload();
            self.inner.write().record_launch(method_text);
        } else {
            error!(
                "[HistoryBooster] 无法找到候选项数据，无法记录启动，candidate_id: {}",
                candidate_id
            );
        }
    }

    /// 基于历史启动数据增强候选项分数
    async fn boost(
        &self,
        candidates: &mut Vec<ScoredCandidate>,
        data: &CachedCandidateData,
        query: &str,
    ) {
        let inner = self.inner.read();
        let settings = self.settings.read();

        for candidate in candidates.iter_mut() {
            let method_text = match data.get_candidate(candidate.candidate_id) {
                Some(sc) => sc.target.payload(),
                None => continue,
            };

            let history_score = inner.calculate_history_score(method_text);
            let recent_habit_score = inner.calculate_recent_habit_score(method_text);
            let temporal_score =
                inner.calculate_temporal_score(method_text, settings.temporal_decay as i64);

            // 动态权重总和
            let dynamic_score = settings.history_weight * history_score
                + settings.recent_habit_weight * recent_habit_score
                + settings.temporal_weight * temporal_score;

            // 基础分抑制因子：基础匹配分过低时抑制动态权重加成
            // 避免高频使用的无关程序挤占低频使用的精准匹配程序
            // 当用户没有输入时，不启用抑制因子
            let base_score = candidate.score;
            let suppression_factor = if query.is_empty() {
                1.0
            } else {
                (base_score / 15.0).clamp(0.0, 1.0)
            };

            let boost_value = dynamic_score * suppression_factor;
            candidate.score += boost_value;

            candidate.detailed_score.push(ScoreDetail {
                score: history_score,
                weight: settings.history_weight,
                description: "历史启动分数".to_string(),
                kind: ScoreDetailKind::Add,
            });
            candidate.detailed_score.push(ScoreDetail {
                score: recent_habit_score,
                weight: settings.recent_habit_weight,
                description: "近期习惯分数".to_string(),
                kind: ScoreDetailKind::Add,
            });
            candidate.detailed_score.push(ScoreDetail {
                score: temporal_score,
                weight: settings.temporal_weight,
                description: "短期热度分数".to_string(),
                kind: ScoreDetailKind::Add,
            });
            candidate.detailed_score.push(ScoreDetail {
                score: suppression_factor,
                weight: 1.0,
                description: "基础分抑制因子".to_string(),
                kind: ScoreDetailKind::Multiply,
            });
        }
    }
}

use crate::plugin_framework::builtin_registry::ScoreBoosterEntry;
use std::sync::Arc;

pub(crate) fn build_history_booster() -> (Arc<dyn Configurable>, Arc<dyn ScoreBooster>) {
    let booster: Arc<dyn ScoreBooster> = Arc::new(HistoryBooster::new());
    let configurable: Arc<dyn Configurable> = booster.clone();
    (configurable, booster)
}

::inventory::submit! {
    ScoreBoosterEntry {
        component_id: "history-booster",
        priority: 0,
        factory: build_history_booster,
    }
}
