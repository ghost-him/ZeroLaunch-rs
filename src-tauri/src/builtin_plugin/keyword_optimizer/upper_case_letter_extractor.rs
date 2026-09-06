use crate::core::config::setting_builders::SchemaBuilder;
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

/// 大写字母提取器的可持久化配置
#[derive(Debug, Clone, Serialize, Deserialize)]
struct UpperCaseLetterExtractorSettings {
    #[serde(rename = "priority", default = "default_priority_40")]
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
    SOURCE_ORIGINAL_NAME.to_string()
}

fn default_priority_40() -> u32 {
    40
}

impl Default for UpperCaseLetterExtractorSettings {
    fn default() -> Self {
        Self {
            priority: default_priority_40(),
            input_source: SOURCE_ORIGINAL_NAME.to_string(),
            producer_id: String::new(),
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
    core: ComponentCore,
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
            core: ComponentCore::new(
                "upper-case-letter-extractor".to_string(),
                t_key!("upper-case-letter-extractor", "name").to_string(),
                t_key!("upper-case-letter-extractor", "description").to_string(),
                ComponentType::KeywordOptimizer,
                70,
            ),
            inner: RwLock::new(UpperCaseLetterExtractorSettings::default()),
        }
    }
}

#[async_trait]
impl Configurable for UpperCaseLetterExtractor {
    fn core(&self) -> &ComponentCore {
        &self.core
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![
            SchemaBuilder::number(
                "priority",
                t_key!("upper-case-letter-extractor", "fields.priority.label"),
                t_key!("upper-case-letter-extractor", "fields.priority.desc"),
            )
            .order(0)
            .default(40.0)
            .min(1.0)
            .max(100.0)
            .step(1.0)
            .build(),
            SchemaBuilder::select(
                "input_source",
                t_key!("upper-case-letter-extractor", "fields.input_source.label"),
                t_key!("upper-case-letter-extractor", "fields.input_source.desc"),
            )
            .options_with_labels(&[
                (
                    SOURCE_ORIGINAL_NAME,
                    t_key!(
                        "upper-case-letter-extractor",
                        "options.input_source.original_name"
                    ),
                ),
                (
                    SOURCE_NORMALIZED_BASE,
                    t_key!(
                        "upper-case-letter-extractor",
                        "options.input_source.normalized_base"
                    ),
                ),
                (
                    SOURCE_REFINED,
                    t_key!(
                        "upper-case-letter-extractor",
                        "options.input_source.refined"
                    ),
                ),
                (
                    SOURCE_OPTIMIZER_OUTPUT,
                    t_key!(
                        "upper-case-letter-extractor",
                        "options.input_source.optimizer_output"
                    ),
                ),
            ])
            .default(SOURCE_ORIGINAL_NAME)
            .order(1)
            .build(),
            SchemaBuilder::text(
                "producer_id",
                t_key!("upper-case-letter-extractor", "fields.producer_id.label"),
                t_key!("upper-case-letter-extractor", "fields.producer_id.desc"),
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
        let parsed: UpperCaseLetterExtractorSettings =
            serde_json::from_value(settings).unwrap_or_default();
        *self.inner.write() = parsed;
        Ok(())
    }
}

#[async_trait]
impl KeywordOptimizer for UpperCaseLetterExtractor {
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
