use crate::core::config::setting_builders::SchemaBuilder;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use zerolaunch_plugin_api::config::{ComponentType, ConfigError, Configurable, SettingDefinition};
use zerolaunch_plugin_api::KeywordOptimizer;

/// Default priority value for LowerCaseConverterSettings.
fn default_priority_30() -> i32 {
    30
}

/// Default uses_context value for LowerCaseConverterSettings.
fn default_uses_context_false() -> bool {
    false
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LowerCaseConverterSettings {
    #[serde(rename = "priority", default = "default_priority_30")]
    priority: i32,
    #[serde(rename = "uses_context", default = "default_uses_context_false")]
    uses_context: bool,
}

impl LowerCaseConverterSettings {
    fn new() -> Self {
        Self {
            priority: 30,
            uses_context: false,
        }
    }

    /// Converts the keyword to lowercase. Returns an empty Vec if the result equals the original.
    fn optimize(&self, keyword: &str) -> Vec<String> {
        let result = keyword.to_lowercase();
        if result == keyword {
            Vec::new()
        } else {
            vec![result]
        }
    }
}

impl Default for LowerCaseConverterSettings {
    fn default() -> Self {
        Self::new()
    }
}

pub struct LowerCaseConverter {
    inner: RwLock<LowerCaseConverterSettings>,
}

impl Default for LowerCaseConverter {
    fn default() -> Self {
        Self::new()
    }
}

impl LowerCaseConverter {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(LowerCaseConverterSettings::new()),
        }
    }
}

impl Configurable for LowerCaseConverter {
    fn component_id(&self) -> &str {
        "lower-case-converter"
    }

    fn component_name(&self) -> &str {
        "小写转换器"
    }

    fn component_description(&self) -> &str {
        "将关键词转换为小写以忽略大小写"
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::KeywordOptimizer
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![
            SchemaBuilder::number("priority", "优先级", "优化器执行优先级，数值越小越先执行")
                .order(0)
                .default(30.0)
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
            .default(false)
            .build(),
        ]
    }

    fn get_settings(&self) -> serde_json::Value {
        serde_json::to_value(self.inner.read().clone()).unwrap_or_default()
    }

    fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        let parsed: LowerCaseConverterSettings =
            serde_json::from_value(settings).unwrap_or_default();
        *self.inner.write() = parsed;
        Ok(())
    }
}

impl KeywordOptimizer for LowerCaseConverter {
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

pub(crate) fn build_lower_case_converter() -> (Arc<dyn Configurable>, Arc<dyn KeywordOptimizer>) {
    let opt: Arc<dyn KeywordOptimizer> = Arc::new(LowerCaseConverter::new());
    let configurable: Arc<dyn Configurable> = opt.clone();
    (configurable, opt)
}

::inventory::submit! {
    KeywordOptimizerEntry {
        component_id: "lower-case-converter",
        priority: 40,
        factory: build_lower_case_converter,
    }
}
