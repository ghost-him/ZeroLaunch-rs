use crate::core::config::setting_builders::SchemaBuilder;
use crate::utils::get_current_time;
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tracing::error;
use zerolaunch_plugin_api::config::{ComponentType, ConfigError, Configurable, SettingDefinition};
use zerolaunch_plugin_api::{
    CachedCandidateData, CandidateId, ScoreBooster, ScoreDetail, ScoredCandidate,
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
    /// 查询亲和度映射: (查询词, launch_method_text) -> QueryAffinityData
    query_affinity_map: DashMap<(String, String), QueryAffinityData>,
}

impl QueryAffinityBoosterInner {
    fn new() -> Self {
        QueryAffinityBoosterInner {
            query_affinity_map: DashMap::new(),
        }
    }

    /// 记录查询-程序启动关联（衰减累积 + 冷却机制）
    fn record_query_launch(
        &mut self,
        query: &str,
        method_text: &str,
        cooldown: i64,
        time_decay: i64,
    ) {
        let query = query.to_lowercase();
        let current_time = get_current_time();
        let key = (query, method_text.to_string());

        self.query_affinity_map
            .entry(key)
            .and_modify(|data| {
                // 冷却机制：检查距离上次记录是否超过冷却时间
                let time_since_last_record = current_time - data.last_record_time;
                if time_since_last_record >= cooldown {
                    // 衰减累积：先对旧的有效次数进行衰减，再加上新的一次
                    let time_diff = current_time - data.last_launch_time;
                    let decay = (-(time_diff as f64) / (time_decay as f64 + 1.0)).exp();

                    data.effective_count = data.effective_count * decay + 1.0;
                    data.last_record_time = current_time;
                }
                // 无论是否在冷却时间内，都更新最后启动时间
                data.last_launch_time = current_time;
            })
            .or_insert(QueryAffinityData::new(current_time));
    }

    /// 计算查询亲和分数（对数缩放 + 时间衰减）
    fn calculate_query_affinity_score(
        &self,
        query: &str,
        method_text: &str,
        time_decay: i64,
    ) -> f64 {
        let key = (query.to_string(), method_text.to_string());

        if let Some(data) = self.query_affinity_map.get(&key) {
            let current_time = get_current_time();
            let time_diff = current_time - data.last_launch_time;

            // 时间衰减因子: exp(-(时间差/时间常数))
            let decay_factor = (-(time_diff as f64) / (time_decay as f64 + 1.0)).exp();

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
            inner: RwLock::new(QueryAffinityBoosterInner::new()),
            settings: RwLock::new(QueryAffinitySettings::default()),
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

    fn component_description(&self) -> &str {
        "根据查询关键词与候选项的匹配程度调整分数"
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::ScoreBooster
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![
            SchemaBuilder::number(
                "query_affinity_weight",
                "查询亲和权重",
                "查询亲和度的权重系数",
            )
            .group("权重配置")
            .order(0)
            .default(3.0)
            .min(0.0)
            .max(20.0)
            .step(0.1)
            .build(),
            SchemaBuilder::number(
                "query_affinity_time_decay",
                "亲和衰减常数(秒)",
                "查询亲和度的时间衰减常数，默认259200秒(3天)",
            )
            .group("衰减配置")
            .order(1)
            .default(259200.0)
            .min(60.0)
            .max(2592000.0)
            .step(60.0)
            .build(),
            SchemaBuilder::number(
                "query_affinity_cooldown",
                "亲和冷却时间(秒)",
                "查询亲和度的冷却时间，防止短时间重复计数，默认15秒",
            )
            .group("冷却配置")
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

    fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        let parsed: QueryAffinitySettings = serde_json::from_value(settings).unwrap_or_default();
        *self.settings.write() = parsed;
        Ok(())
    }
}

impl ScoreBooster for QueryAffinityBooster {
    /// 记录查询-候选项启动关联
    fn record(&self, candidate_id: CandidateId, data: &CachedCandidateData, query: &str) {
        if query.trim().is_empty() {
            return;
        }
        if let Some(search_candidate) = data.get_candidate(candidate_id) {
            let method_text = search_candidate.target.payload();
            let settings = self.settings.read();
            self.inner.write().record_query_launch(
                query,
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
        let settings = self.settings.read();

        for candidate in candidates.iter_mut() {
            let method_text = match data.get_candidate(candidate.candidate_id) {
                Some(sc) => sc.target.payload(),
                None => continue,
            };

            let affinity_score = inner.calculate_query_affinity_score(
                query,
                method_text,
                settings.query_affinity_time_decay as i64,
            );
            let boost_value = settings.query_affinity_weight * affinity_score;
            candidate.score += boost_value;

            candidate.detailed_score.push(ScoreDetail {
                score: affinity_score,
                weight: settings.query_affinity_weight,
                description: "查询亲和分数".to_string(),
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
