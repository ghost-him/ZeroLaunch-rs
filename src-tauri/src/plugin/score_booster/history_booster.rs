use crate::plugin_system::cached_candidate::CachedCandidateData;
use crate::plugin_system::types::{
    CandidateId, ComponentType, ConfigError, Configurable, FieldDefinition, ScoreBooster,
    ScoreDetail, ScoredCandidate, SettingDefinition, SettingType,
};
use crate::utils::{generate_current_date, get_current_time, is_date_current};
use dashmap::DashMap;
use parking_lot::RwLock;
use std::collections::VecDeque;
use tracing::error;

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
    /// 历史总分权重系数
    history_weight: f64,
    /// 近期习惯权重系数
    recent_habit_weight: f64,
    /// 短期热度权重系数
    temporal_weight: f64,
    /// 短期热度衰减常数（秒）
    temporal_decay: i64,
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
            history_weight: 0.8,
            recent_habit_weight: 1.5,
            temporal_weight: 0.5,
            temporal_decay: 10800,
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
    fn calculate_temporal_score(&self, method_text: &str) -> f64 {
        if let Some(last_launch_time) = self.latest_launch_time.get(method_text) {
            let current_time = get_current_time();
            let time_diff = current_time - *last_launch_time;
            let k = 6.0;
            k / (1.0 + (time_diff as f64) / (self.temporal_decay as f64 + 1.0))
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
    inner: RwLock<HistoryBoosterInner>,
    settings: RwLock<serde_json::Value>,
}

impl Default for HistoryBooster {
    fn default() -> Self {
        Self::new()
    }
}

impl HistoryBooster {
    pub fn new() -> Self {
        HistoryBooster {
            inner: RwLock::new(HistoryBoosterInner::new()),
            settings: RwLock::new(serde_json::Value::Null),
        }
    }
}

impl Configurable for HistoryBooster {
    fn component_id(&self) -> &str {
        "history-booster"
    }

    fn component_name(&self) -> &str {
        "历史记录增强器"
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::ScoreBooster
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![
            SettingDefinition {
                field: FieldDefinition {
                    key: "history_weight".to_string(),
                    label: "历史权重".to_string(),
                    description: "历史总启动次数的权重系数".to_string(),
                    setting_type: SettingType::Number {
                        min: Some(0.0),
                        max: Some(10.0),
                        step: Some(0.1),
                    },
                    default_value: serde_json::json!(0.8),
                    visible: true,
                    editable: true,
                },
                group: Some("权重配置".to_string()),
                order: 0,
                config_action: None,
            },
            SettingDefinition {
                field: FieldDefinition {
                    key: "recent_habit_weight".to_string(),
                    label: "近期习惯权重".to_string(),
                    description: "最近7天启动习惯的权重系数".to_string(),
                    setting_type: SettingType::Number {
                        min: Some(0.0),
                        max: Some(10.0),
                        step: Some(0.1),
                    },
                    default_value: serde_json::json!(1.5),
                    visible: true,
                    editable: true,
                },
                group: Some("权重配置".to_string()),
                order: 1,
                config_action: None,
            },
            SettingDefinition {
                field: FieldDefinition {
                    key: "temporal_weight".to_string(),
                    label: "短期热度权重".to_string(),
                    description: "短期热度的权重系数".to_string(),
                    setting_type: SettingType::Number {
                        min: Some(0.0),
                        max: Some(10.0),
                        step: Some(0.1),
                    },
                    default_value: serde_json::json!(0.5),
                    visible: true,
                    editable: true,
                },
                group: Some("权重配置".to_string()),
                order: 2,
                config_action: None,
            },
            SettingDefinition {
                field: FieldDefinition {
                    key: "temporal_decay".to_string(),
                    label: "热度衰减常数(秒)".to_string(),
                    description: "短期热度的衰减时间常数，默认10800秒(3小时)".to_string(),
                    setting_type: SettingType::Number {
                        min: Some(60.0),
                        max: Some(86400.0),
                        step: Some(60.0),
                    },
                    default_value: serde_json::json!(10800),
                    visible: true,
                    editable: true,
                },
                group: Some("衰减配置".to_string()),
                order: 3,
                config_action: None,
            },
        ]
    }

    fn get_settings(&self) -> serde_json::Value {
        self.settings.read().clone()
    }

    fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        let mut inner = self.inner.write();
        if let Some(v) = settings.get("history_weight").and_then(|v| v.as_f64()) {
            inner.history_weight = v;
        }
        if let Some(v) = settings.get("recent_habit_weight").and_then(|v| v.as_f64()) {
            inner.recent_habit_weight = v;
        }
        if let Some(v) = settings.get("temporal_weight").and_then(|v| v.as_f64()) {
            inner.temporal_weight = v;
        }
        if let Some(v) = settings.get("temporal_decay").and_then(|v| v.as_f64()) {
            inner.temporal_decay = v as i64;
        }
        *self.settings.write() = settings;
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

        for candidate in candidates.iter_mut() {
            let method_text = match data.get_candidate(candidate.candidate_id) {
                Some(sc) => sc.target.payload(),
                None => continue,
            };

            let history_score = inner.calculate_history_score(method_text);
            let recent_habit_score = inner.calculate_recent_habit_score(method_text);
            let temporal_score = inner.calculate_temporal_score(method_text);

            // 动态权重总和
            let dynamic_score = inner.history_weight * history_score
                + inner.recent_habit_weight * recent_habit_score
                + inner.temporal_weight * temporal_score;

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
                weight: inner.history_weight,
                description: "历史启动分数".to_string(),
            });
            candidate.detailed_score.push(ScoreDetail {
                score: recent_habit_score,
                weight: inner.recent_habit_weight,
                description: "近期习惯分数".to_string(),
            });
            candidate.detailed_score.push(ScoreDetail {
                score: temporal_score,
                weight: inner.temporal_weight,
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
