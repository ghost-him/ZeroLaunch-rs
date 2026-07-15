use crate::core::config::setting_builders::SchemaBuilder;
use crate::utils::{generate_current_date, get_current_time, is_date_current};
use async_trait::async_trait;
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use tracing::error;
use zerolaunch_plugin_api::config::{
    ComponentCore, ComponentType, ConfigError, Configurable, SettingDefinition,
};
use zerolaunch_plugin_api::{
    CachedCandidateData, CandidateId, ScoreBooster, ScoreDetail, ScoredCandidate,
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

/// 历史记录增强器内部实现
#[derive(Debug)]
struct HistoryBoosterInner {
    /// 最近7天的启动次数记录
    launch_time: VecDeque<DashMap<String, u64>>,
    /// 历史总启动次数
    history_launch_time: DashMap<String, u64>,
    /// 上次更新日期
    last_update_data: String,
    /// 最近一次启动时间（时间戳）
    latest_launch_time: DashMap<String, i64>,
}

impl HistoryBoosterInner {
    fn new() -> Self {
        let mut deque = VecDeque::new();
        deque.push_front(DashMap::new());
        HistoryBoosterInner {
            launch_time: deque,
            history_launch_time: DashMap::new(),
            last_update_data: generate_current_date(),
            latest_launch_time: DashMap::new(),
        }
    }

    /// 记录程序启动，更新所有统计数据
    fn record_launch(&mut self, method_text: &str) {
        // 确保日期信息已更新
        self.update_launch_info();

        let method_key = method_text.to_string();
        let current_time = get_current_time();

        // 更新今日启动次数
        self.launch_time[0]
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
        self.launch_time.iter().for_each(|day| {
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

    fn update_launch_info(&mut self) {
        if !is_date_current(&self.last_update_data) {
            self.launch_time.push_front(DashMap::new());
            if self.launch_time.len() > 7 {
                self.launch_time.pop_back();
            }
            self.last_update_data = generate_current_date();
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
                "历史记录增强器".to_string(),
                "根据历史选择频率提升常用候选项的排名".to_string(),
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
            SchemaBuilder::number("history_weight", "历史权重", "历史总启动次数的权重系数")
                .group("权重配置")
                .order(0)
                .default(0.8)
                .min(0.0)
                .max(10.0)
                .step(0.1)
                .build(),
            SchemaBuilder::number(
                "recent_habit_weight",
                "近期习惯权重",
                "最近7天启动习惯的权重系数",
            )
            .group("权重配置")
            .order(1)
            .default(1.5)
            .min(0.0)
            .max(10.0)
            .step(0.1)
            .build(),
            SchemaBuilder::number("temporal_weight", "短期热度权重", "短期热度的权重系数")
                .group("权重配置")
                .order(2)
                .default(0.5)
                .min(0.0)
                .max(10.0)
                .step(0.1)
                .build(),
            SchemaBuilder::number(
                "temporal_decay",
                "热度衰减常数(秒)",
                "短期热度的衰减时间常数，默认10800秒(3小时)",
            )
            .group("衰减配置")
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

    fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        let parsed: HistoryBoosterSettings = serde_json::from_value(settings).unwrap_or_default();
        *self.settings.write() = parsed;
        Ok(())
    }
}

impl ScoreBooster for HistoryBooster {
    /// 记录候选项被选中启动
    fn record(&self, candidate_id: CandidateId, data: &CachedCandidateData, _query: &str) {
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
    fn boost(
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
            });
            candidate.detailed_score.push(ScoreDetail {
                score: recent_habit_score,
                weight: settings.recent_habit_weight,
                description: "近期习惯分数".to_string(),
            });
            candidate.detailed_score.push(ScoreDetail {
                score: temporal_score,
                weight: settings.temporal_weight,
                description: "短期热度分数".to_string(),
            });
            candidate.detailed_score.push(ScoreDetail {
                score: suppression_factor,
                weight: 1.0,
                description: "基础分抑制因子".to_string(),
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
