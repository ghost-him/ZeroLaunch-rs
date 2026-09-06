use crate::core::config::setting_builders::SchemaBuilder;
use crate::utils::collapse_repeated_spaces;
use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use zerolaunch_plugin_api::config::{
    ComponentCore, ComponentType, ConfigError, Configurable, SettingDefinition,
};
use zerolaunch_plugin_api::{KeywordInputSource, KeywordOptimizer};

/// 空格规范化器的可持久化配置
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SpaceNormalizerSettings {
    #[serde(rename = "priority", default = "default_priority_20")]
    priority: u32,
}

fn default_priority_20() -> u32 {
    20
}

impl SpaceNormalizerSettings {
    fn new() -> Self {
        Self { priority: 20 }
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
        vec![SchemaBuilder::number(
            "priority",
            t_key!("space-normalizer", "fields.priority.label"),
            t_key!("space-normalizer", "fields.priority.desc"),
        )
        .order(0)
        .default(20.0)
        .min(1.0)
        .max(100.0)
        .step(1.0)
        .build()]
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
        KeywordInputSource::NormalizedBase
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
