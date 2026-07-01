use crate::core::config::setting_builders::SchemaBuilder;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use zerolaunch_plugin_api::config::{ComponentType, ConfigError, Configurable, SettingDefinition};
use zerolaunch_plugin_api::KeywordOptimizer;

/// Default priority value for FirstLetterExtractorSettings.
fn default_priority_50() -> i32 {
    50
}

/// Default uses_context value for FirstLetterExtractorSettings.
fn default_uses_context_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FirstLetterExtractorSettings {
    #[serde(rename = "priority", default = "default_priority_50")]
    priority: i32,
    #[serde(rename = "uses_context", default = "default_uses_context_true")]
    uses_context: bool,
}

impl FirstLetterExtractorSettings {
    fn new() -> Self {
        Self {
            priority: 50,
            uses_context: true,
        }
    }

    /// Extracts the first letter of each whitespace-separated word in the input string.
    fn get_first_letters(&self, s: &str) -> String {
        s.split_whitespace()
            .filter_map(|word| word.chars().next())
            .collect()
    }

    /// Generates a keyword variant consisting of the first letters of each word.
    /// Returns an empty Vec if the result equals the original keyword or is empty.
    fn optimize(&self, keyword: &str) -> Vec<String> {
        let result = self.get_first_letters(keyword);
        if result.is_empty() || result == keyword {
            Vec::new()
        } else {
            vec![result]
        }
    }
}

impl Default for FirstLetterExtractorSettings {
    fn default() -> Self {
        Self::new()
    }
}

pub struct FirstLetterExtractor {
    inner: RwLock<FirstLetterExtractorSettings>,
}

impl Default for FirstLetterExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl FirstLetterExtractor {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(FirstLetterExtractorSettings::new()),
        }
    }
}

impl Configurable for FirstLetterExtractor {
    fn component_id(&self) -> &str {
        "first-letter-extractor"
    }

    fn component_name(&self) -> &str {
        "首字母提取器"
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::KeywordOptimizer
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![
            SchemaBuilder::number("priority", "优先级", "优化器执行优先级，数值越小越先执行")
                .order(0)
                .default(50.0)
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
        let parsed: FirstLetterExtractorSettings =
            serde_json::from_value(settings).unwrap_or_default();
        *self.inner.write() = parsed;
        Ok(())
    }
}

impl KeywordOptimizer for FirstLetterExtractor {
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

use crate::plugin_system::builtin_registry::KeywordOptimizerEntry;
use std::sync::Arc;

pub(crate) fn build_first_letter_extractor() -> (Arc<dyn Configurable>, Arc<dyn KeywordOptimizer>) {
    let opt: Arc<dyn KeywordOptimizer> = Arc::new(FirstLetterExtractor::new());
    let configurable: Arc<dyn Configurable> = opt.clone();
    (configurable, opt)
}

::inventory::submit! {
    KeywordOptimizerEntry {
        component_id: "first-letter-extractor",
        priority: 60,
        factory: build_first_letter_extractor,
    }
}
