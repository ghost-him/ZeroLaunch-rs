use crate::core::config::setting_builders::SchemaBuilder;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use zerolaunch_plugin_api::config::{ComponentType, ConfigError, Configurable, SettingDefinition};
use zerolaunch_plugin_api::KeywordOptimizer;

/// 大写字母提取器的可持久化配置
#[derive(Debug, Clone, Serialize, Deserialize)]
struct UpperCaseLetterExtractorSettings {
    #[serde(rename = "priority", default = "default_priority_40")]
    priority: i32,
    #[serde(rename = "uses_context", default = "default_uses_context_true")]
    uses_context: bool,
}

fn default_priority_40() -> i32 {
    40
}

fn default_uses_context_true() -> bool {
    true
}

impl Default for UpperCaseLetterExtractorSettings {
    fn default() -> Self {
        Self {
            priority: default_priority_40(),
            uses_context: default_uses_context_true(),
        }
    }
}

impl UpperCaseLetterExtractorSettings {
    /// 提取输入字符串中的所有 ASCII 大写字母并转为小写。
    /// 若包含非 ASCII 字符则返回空字符串（仅对纯英文输入生效）。
    fn get_upper_case_latter(&self, input_text: &str) -> String {
        let mut result = String::new();

        for c in input_text.chars() {
            if c.is_ascii_uppercase() {
                result.push(c);
            }
            if !c.is_ascii() {
                result.clear();
                break;
            }
        }

        result.to_lowercase()
    }

    /// 对关键词执行优化：提取大写字母作为缩写关键词
    fn optimize(&self, keyword: &str) -> Vec<String> {
        let result = self.get_upper_case_latter(keyword);
        if result.is_empty() {
            Vec::new()
        } else {
            vec![result]
        }
    }
}

pub struct UpperCaseLetterExtractor {
    inner: RwLock<UpperCaseLetterExtractorSettings>,
}

impl Default for UpperCaseLetterExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl UpperCaseLetterExtractor {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(UpperCaseLetterExtractorSettings::default()),
        }
    }
}

impl Configurable for UpperCaseLetterExtractor {
    fn component_id(&self) -> &str {
        "upper-case-letter-extractor"
    }

    fn component_name(&self) -> &str {
        "大写字母提取器"
    }

    fn component_description(&self) -> &str {
        "提取大写字母以支持缩写搜索"
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::KeywordOptimizer
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![
            SchemaBuilder::number("priority", "优先级", "优化器执行优先级，数值越小越先执行")
                .order(0)
                .default(40.0)
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
        let parsed: UpperCaseLetterExtractorSettings =
            serde_json::from_value(settings).unwrap_or_default();
        *self.inner.write() = parsed;
        Ok(())
    }
}

impl KeywordOptimizer for UpperCaseLetterExtractor {
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

pub(crate) fn build_upper_case_letter_extractor(
) -> (Arc<dyn Configurable>, Arc<dyn KeywordOptimizer>) {
    let opt: Arc<dyn KeywordOptimizer> = Arc::new(UpperCaseLetterExtractor::new());
    let configurable: Arc<dyn Configurable> = opt.clone();
    (configurable, opt)
}

::inventory::submit! {
    KeywordOptimizerEntry {
        component_id: "upper-case-letter-extractor",
        priority: 70,
        factory: build_upper_case_letter_extractor,
    }
}
