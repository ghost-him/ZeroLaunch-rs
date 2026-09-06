use crate::core::config::setting_builders::SchemaBuilder;
use crate::utils::collapse_repeated_spaces;
use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use zerolaunch_plugin_api::config::{
    ComponentCore, ComponentType, ConfigError, Configurable, SettingDefinition,
};
use zerolaunch_plugin_api::{KeywordInputSource, KeywordOptimizer};

use super::input_source::{
    resolve_input_source, SOURCE_NORMALIZED_BASE, SOURCE_OPTIMIZER_OUTPUT, SOURCE_ORIGINAL_NAME,
    SOURCE_REFINED,
};

/// 空格规范化器的可持久化配置
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SpaceNormalizerSettings {
    #[serde(rename = "priority", default = "default_priority_20")]
    priority: u32,
    /// 输入来源（KeywordInputSource 的扁平 snake_case 值，见 input_source 模块常量）。
    /// 决定本优化器消费哪一层关键词产物。
    #[serde(rename = "input_source", default = "default_input_source")]
    input_source: String,
    /// 被引用生产者优化器的 component_id；仅当 input_source = optimizer_output 时生效。
    #[serde(rename = "producer_id", default)]
    producer_id: String,
}

fn default_input_source() -> String {
    SOURCE_NORMALIZED_BASE.to_string()
}

fn default_priority_20() -> u32 {
    20
}

impl SpaceNormalizerSettings {
    fn new() -> Self {
        Self {
            priority: 20,
            input_source: SOURCE_NORMALIZED_BASE.to_string(),
            producer_id: String::new(),
        }
    }

    /// Removes leading spaces and collapses consecutive spaces into a single space.
    fn optimize(&self, keyword: &str) -> Vec<String> {
        let result = collapse_repeated_spaces(keyword);
        if result.is_empty() || result == keyword {
            Vec::new()
        } else {
            vec![result]
        }
    }
}

impl Default for SpaceNormalizerSettings {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SpaceNormalizer {
    core: ComponentCore,
    inner: RwLock<SpaceNormalizerSettings>,
}

impl Default for SpaceNormalizer {
    fn default() -> Self {
        Self::new()
    }
}

impl SpaceNormalizer {
    pub fn new() -> Self {
        Self {
            core: ComponentCore::new(
                "space-normalizer".to_string(),
                t_key!("space-normalizer", "name").to_string(),
                t_key!("space-normalizer", "description").to_string(),
                ComponentType::KeywordOptimizer,
                30,
            ),
            inner: RwLock::new(SpaceNormalizerSettings::new()),
        }
    }
}

#[async_trait]
impl Configurable for SpaceNormalizer {
    fn core(&self) -> &ComponentCore {
        &self.core
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![
            SchemaBuilder::number(
                "priority",
                t_key!("space-normalizer", "fields.priority.label"),
                t_key!("space-normalizer", "fields.priority.desc"),
            )
            .order(0)
            .default(20.0)
            .min(1.0)
            .max(100.0)
            .step(1.0)
            .build(),
            SchemaBuilder::select(
                "input_source",
                t_key!("space-normalizer", "fields.input_source.label"),
                t_key!("space-normalizer", "fields.input_source.desc"),
            )
            .options_with_labels(&[
                (
                    SOURCE_ORIGINAL_NAME,
                    t_key!("space-normalizer", "options.input_source.original_name"),
                ),
                (
                    SOURCE_NORMALIZED_BASE,
                    t_key!("space-normalizer", "options.input_source.normalized_base"),
                ),
                (
                    SOURCE_REFINED,
                    t_key!("space-normalizer", "options.input_source.refined"),
                ),
                (
                    SOURCE_OPTIMIZER_OUTPUT,
                    t_key!("space-normalizer", "options.input_source.optimizer_output"),
                ),
            ])
            .default(SOURCE_NORMALIZED_BASE)
            .order(1)
            .build(),
            SchemaBuilder::text(
                "producer_id",
                t_key!("space-normalizer", "fields.producer_id.label"),
                t_key!("space-normalizer", "fields.producer_id.desc"),
            )
            .default("")
            .order(2)
            .visible_when("input_source", SOURCE_OPTIMIZER_OUTPUT)
            .build(),
        ]
    }

    fn get_settings(&self) -> serde_json::Value {
        serde_json::to_value(self.inner.read().clone()).unwrap_or_default()
    }

    async fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        let parsed: SpaceNormalizerSettings = serde_json::from_value(settings).unwrap_or_default();
        *self.inner.write() = parsed;
        Ok(())
    }
}

#[async_trait]
impl KeywordOptimizer for SpaceNormalizer {
    async fn optimize(&self, keyword: &str) -> Vec<String> {
        self.inner.read().optimize(keyword)
    }

    fn input_source(&self) -> KeywordInputSource {
        let settings = self.inner.read();
        resolve_input_source(&settings.input_source, &settings.producer_id)
    }

    fn get_priority(&self) -> u32 {
        self.inner.read().priority
    }
}

use crate::plugin_framework::builtin_registry::KeywordOptimizerEntry;
use std::sync::Arc;

pub(crate) fn build_space_normalizer() -> (Arc<dyn Configurable>, Arc<dyn KeywordOptimizer>) {
    let opt: Arc<dyn KeywordOptimizer> = Arc::new(SpaceNormalizer::new());
    let configurable: Arc<dyn Configurable> = opt.clone();
    (configurable, opt)
}

::inventory::submit! {
    KeywordOptimizerEntry {
        component_id: "space-normalizer",
        priority: 30,
        factory: build_space_normalizer,
    }
}
