use crate::plugin_system::cached_candidate::CachedCandidateData;
use crate::plugin_system::types::{
    ComponentType, ConfigError, Configurable, FieldDefinition, ScoreBooster, ScoreDetail,
    ScoredCandidate, SettingDefinition, SettingType,
};
use crate::utils::get_current_time;
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tracing::error;
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

/// 查询亲和度增强器内部实现
#[derive(Debug)]
struct QueryAffinityBoosterInner {
    /// 查询亲和度映射: (查询词, launch_method_text) -> QueryAffinityData
    query_affinity_map: DashMap<(String, String), QueryAffinityData>,
    /// 查询亲和权重系数
    query_affinity_weight: f64,
    /// 查询亲和时间衰减常数（秒）
    query_affinity_time_decay: i64,
    /// 查询亲和冷却时间（秒）
    query_affinity_cooldown: i64,
}

impl QueryAffinityBoosterInner {
    fn new() -> Self {
        QueryAffinityBoosterInner {
            query_affinity_map: DashMap::new(),
            query_affinity_weight: 3.0,
            query_affinity_time_decay: 259200,
            query_affinity_cooldown: 15,
        }
    }

    /// 记录查询-程序启动关联（衰减累积 + 冷却机制）
    fn record_query_launch(&mut self, query: &str, method_text: &str) {
        let query = query.to_lowercase();
        // 这里的归一化由外部保证
        // let query = remove_repeated_space(&query);
        let current_time = get_current_time();
        let key = (query, method_text.to_string());

        self.query_affinity_map
            .entry(key)
            .and_modify(|data| {
                // 冷却机制：检查距离上次记录是否超过冷却时间
                let time_since_last_record = current_time - data.last_record_time;
                if time_since_last_record >= self.query_affinity_cooldown {
                    // 衰减累积：先对旧的有效次数进行衰减，再加上新的一次
                    let time_diff = current_time - data.last_launch_time;
                    let decay =
                        (-(time_diff as f64) / (self.query_affinity_time_decay as f64 + 1.0)).exp();

                    data.effective_count = data.effective_count * decay + 1.0;
                    data.last_record_time = current_time;
                }
                // 无论是否在冷却时间内，都更新最后启动时间
                data.last_launch_time = current_time;
            })
            .or_insert(QueryAffinityData::new(current_time));
    }

    /// 计算查询亲和分数（对数缩放 + 时间衰减）
    fn calculate_query_affinity_score(&self, query: &str, method_text: &str) -> f64 {
        let key = (query.to_string(), method_text.to_string());

        if let Some(data) = self.query_affinity_map.get(&key) {
            let current_time = get_current_time();
            let time_diff = current_time - data.last_launch_time;

            // 时间衰减因子: exp(-(时间差/时间常数))
            let decay_factor =
                (-(time_diff as f64) / (self.query_affinity_time_decay as f64 + 1.0)).exp();

            // 计算当前有效次数（应用衰减）
            let current_effective_count = data.effective_count * decay_factor;

            // 使用对数缩放，避免分数过大
            (current_effective_count).ln_1p() * 10.0
        } else {
            0.0
        }
    }
}

/// 查询亲和度增强器 - 基于查询词与候选项的关联关系对候选项进行分数增强
#[derive(Debug)]
pub struct QueryAffinityBooster {
    inner: RwLock<QueryAffinityBoosterInner>,
    settings: RwLock<serde_json::Value>,
}

impl Default for QueryAffinityBooster {
    fn default() -> Self {
        Self::new()
    }
}

impl QueryAffinityBooster {
    pub fn new() -> Self {
        QueryAffinityBooster {
            inner: RwLock::new(QueryAffinityBoosterInner::new()),
            settings: RwLock::new(serde_json::Value::Null),
        }
    }
}

impl Configurable for QueryAffinityBooster {
    fn component_id(&self) -> &str {
        "query-affinity-booster"
    }

    fn component_name(&self) -> &str {
        "查询亲和度增强器"
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::ScoreBooster
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![
            SettingDefinition {
                field: FieldDefinition {
                    key: "query_affinity_weight".to_string(),
                    label: "查询亲和权重".to_string(),
                    description: "查询亲和度的权重系数".to_string(),
                    setting_type: SettingType::Number {
                        min: Some(0.0),
                        max: Some(20.0),
                        step: Some(0.1),
                    },
                    default_value: serde_json::json!(3.0),
                    visible: true,
                    editable: true,
                },
                group: Some("权重配置".to_string()),
                order: 0,
                config_action: None,
            },
            SettingDefinition {
                field: FieldDefinition {
                    key: "query_affinity_time_decay".to_string(),
                    label: "亲和衰减常数(秒)".to_string(),
                    description: "查询亲和度的时间衰减常数，默认259200秒(3天)".to_string(),
                    setting_type: SettingType::Number {
                        min: Some(60.0),
                        max: Some(2592000.0),
                        step: Some(60.0),
                    },
                    default_value: serde_json::json!(259200),
                    visible: true,
                    editable: true,
                },
                group: Some("衰减配置".to_string()),
                order: 1,
                config_action: None,
            },
            SettingDefinition {
                field: FieldDefinition {
                    key: "query_affinity_cooldown".to_string(),
                    label: "亲和冷却时间(秒)".to_string(),
                    description: "查询亲和度的冷却时间，防止短时间重复计数，默认15秒".to_string(),
                    setting_type: SettingType::Number {
                        min: Some(1.0),
                        max: Some(300.0),
                        step: Some(1.0),
                    },
                    default_value: serde_json::json!(15),
                    visible: true,
                    editable: true,
                },
                group: Some("冷却配置".to_string()),
                order: 2,
                config_action: None,
            },
        ]
    }

    fn get_settings(&self) -> serde_json::Value {
        self.settings.read().clone()
    }

    fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        let mut inner = self.inner.write();
        if let Some(v) = settings
            .get("query_affinity_weight")
            .and_then(|v| v.as_f64())
        {
            inner.query_affinity_weight = v;
        }
        if let Some(v) = settings
            .get("query_affinity_time_decay")
            .and_then(|v| v.as_f64())
        {
            inner.query_affinity_time_decay = v as i64;
        }
        if let Some(v) = settings
            .get("query_affinity_cooldown")
            .and_then(|v| v.as_f64())
        {
            inner.query_affinity_cooldown = v as i64;
        }
        *self.settings.write() = settings;
        Ok(())
    }
}

impl ScoreBooster for QueryAffinityBooster {
    /// 记录查询-候选项启动关联
    fn record(&self, candidate: &ScoredCandidate, data: &CachedCandidateData, query: &str) {
        if query.trim().is_empty() {
            return;
        }
        if let Some(search_candidate) = data.get_candidate(candidate.candidate_id) {
            let method_text = search_candidate.launch_method.payload();
            self.inner.write().record_query_launch(query, method_text);
        } else {
            error!(
                "[QueryAffinityBooster] 无法找到候选项数据，无法记录查询启动关联，candidate_id: {}",
                candidate.candidate_id
            );
        }
    }

    /// 基于查询亲和度增强候选项分数
    fn boost(
        &self,
        candidates: &mut Vec<ScoredCandidate>,
        data: &CachedCandidateData,
        query: &str,
    ) {
        if query.is_empty() {
            return;
        }

        let inner = self.inner.read();

        for candidate in candidates.iter_mut() {
            let method_text = match data.get_candidate(candidate.candidate_id) {
                Some(sc) => sc.launch_method.payload(),
                None => continue,
            };

            let affinity_score = inner.calculate_query_affinity_score(query, method_text);
            let boost_value = inner.query_affinity_weight * affinity_score;
            candidate.score += boost_value;

            candidate.detailed_score.push(ScoreDetail {
                score: affinity_score,
                weight: inner.query_affinity_weight,
                description: "查询亲和分数".to_string(),
            });
        }
    }
}
