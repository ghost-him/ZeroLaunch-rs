use crate::core::config::setting_builders::SchemaBuilder;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use zerolaunch_plugin_api::config::{ComponentType, ConfigError, Configurable, SettingDefinition};
use zerolaunch_plugin_api::KeywordOptimizer;

/// Default priority value for SpaceNormalizerSettings.
fn default_priority_20() -> i32 {
    20
}

/// Default uses_context value for SpaceNormalizerSettings.
fn default_uses_context_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SpaceNormalizerSettings {
    #[serde(rename = "priority", default = "default_priority_20")]
    priority: i32,
    #[serde(rename = "uses_context", default = "default_uses_context_true")]
    uses_context: bool,
}

impl SpaceNormalizerSettings {
    fn new() -> Self {
        Self {
            priority: 20,
            uses_context: true,
        }
    }

    /// Removes leading spaces and collapses consecutive spaces into a single space.
    fn remove_repeated_space(&self, input_text: &str) -> String {
        let mut result = String::new();
        let mut is_space = false;

        for c in input_text.chars() {
            if c != ' ' {
                result.push(c);
                is_space = false;
            } else if !is_space && !result.is_empty() {
                result.push(c);
                is_space = true;
            } else {
                is_space = true;
            }
        }

        if result.ends_with(' ') {
            result.pop();
        }

        result
    }

    /// Normalizes whitespace in the keyword. Returns an empty Vec if the result equals the original.
    fn optimize(&self, keyword: &str) -> Vec<String> {
        let result = self.remove_repeated_space(keyword);
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
            inner: RwLock::new(SpaceNormalizerSettings::new()),
        }
    }
}

impl Configurable for SpaceNormalizer {
    fn component_id(&self) -> &str {
        "space-normalizer"
    }

    fn component_name(&self) -> &str {
        "空格规范化器"
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::KeywordOptimizer
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![
            SchemaBuilder::number("priority", "优先级", "优化器执行优先级，数值越小越先执行")
                .order(0)
                .default(20.0)
                .min(1.0)
                .max(100.0)
                .step(1.0)
                .build(),
            SchemaBuilder::boolean(
                "uses_context",
                "使用上下文",
                "是否对所有已累积的关键词进行优化，而非仅对原始名称优化",
            )
            .order(1)
            .default(true)
            .build(),
        ]
    }

    fn get_settings(&self) -> serde_json::Value {
        serde_json::to_value(self.inner.read().clone()).unwrap_or_default()
    }

    fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        let parsed: SpaceNormalizerSettings = serde_json::from_value(settings).unwrap_or_default();
        *self.inner.write() = parsed;
        Ok(())
    }
}

impl KeywordOptimizer for SpaceNormalizer {
    fn optimize(&self, keyword: &str) -> Vec<String> {
        self.inner.read().optimize(keyword)
    }

    fn uses_context(&self) -> bool {
        self.inner.read().uses_context
    }

    fn get_priority(&self) -> i32 {
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
