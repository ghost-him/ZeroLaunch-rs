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

/// Default priority value for FirstLetterExtractorSettings.
fn default_priority_50() -> u32 {
    50
}

/// 首字母提取器的可持久化配置
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FirstLetterExtractorSettings {
    #[serde(rename = "priority", default = "default_priority_50")]
    priority: u32,
    /// 输入来源（KeywordInputSource 的扁平 snake_case 值，见 input_source 模块常量）。
    /// 决定本优化器消费哪一层关键词产物。
    #[serde(rename = "input_source", default = "default_input_source")]
    input_source: String,
    /// 被引用生产者优化器的 component_id；仅当 input_source = optimizer_output 时生效。
    /// 默认 pinyin-converter（与 new()/schema 默认一致，防止旧配置缺字段时空引用失效）。
    #[serde(rename = "producer_id", default = "default_producer_id")]
    producer_id: String,
}

fn default_input_source() -> String {
    // 首字母缩写消费拼音转换器的登记输出（QQ音乐→qq yin le→qyl、VSC→vsc）；
    // 不直接吃归一化基/派生词，避免对单 token 取首字符产生无意义单字符（vsc→v、qq→q）。
    SOURCE_OPTIMIZER_OUTPUT.to_string()
}

fn default_producer_id() -> String {
    "pinyin-converter".to_string()
}

impl FirstLetterExtractorSettings {
    fn new() -> Self {
        Self {
            priority: 50,
            input_source: SOURCE_OPTIMIZER_OUTPUT.to_string(),
            producer_id: default_producer_id(),
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
    core: ComponentCore,
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
            core: ComponentCore::new(
                "first-letter-extractor".to_string(),
                t_key!("first-letter-extractor", "name").to_string(),
                t_key!("first-letter-extractor", "description").to_string(),
                ComponentType::KeywordOptimizer,
                60,
            ),
            inner: RwLock::new(FirstLetterExtractorSettings::new()),
        }
    }
}

#[async_trait]
impl Configurable for FirstLetterExtractor {
    fn core(&self) -> &ComponentCore {
        &self.core
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![
            SchemaBuilder::number(
                "priority",
                t_key!("first-letter-extractor", "fields.priority.label"),
                t_key!("first-letter-extractor", "fields.priority.desc"),
            )
            .order(0)
            .default(50.0)
            .min(1.0)
            .max(100.0)
            .step(1.0)
            .build(),
            SchemaBuilder::select(
                "input_source",
                t_key!("first-letter-extractor", "fields.input_source.label"),
                t_key!("first-letter-extractor", "fields.input_source.desc"),
            )
            .options_with_labels(&[
                (
                    SOURCE_ORIGINAL_NAME,
                    t_key!(
                        "first-letter-extractor",
                        "options.input_source.original_name"
                    ),
                ),
                (
                    SOURCE_NORMALIZED_BASE,
                    t_key!(
                        "first-letter-extractor",
                        "options.input_source.normalized_base"
                    ),
                ),
                (
                    SOURCE_REFINED,
                    t_key!("first-letter-extractor", "options.input_source.refined"),
                ),
                (
                    SOURCE_OPTIMIZER_OUTPUT,
                    t_key!(
                        "first-letter-extractor",
                        "options.input_source.optimizer_output"
                    ),
                ),
            ])
            .default(SOURCE_OPTIMIZER_OUTPUT)
            .order(1)
            .build(),
            SchemaBuilder::text(
                "producer_id",
                t_key!("first-letter-extractor", "fields.producer_id.label"),
                t_key!("first-letter-extractor", "fields.producer_id.desc"),
            )
            .default("pinyin-converter")
            .order(2)
            .visible_when("input_source", SOURCE_OPTIMIZER_OUTPUT)
            .build(),
        ]
    }

    fn get_settings(&self) -> serde_json::Value {
        serde_json::to_value(self.inner.read().clone()).unwrap_or_default()
    }

    async fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        let parsed: FirstLetterExtractorSettings =
            serde_json::from_value(settings).unwrap_or_default();
        *self.inner.write() = parsed;
        Ok(())
    }
}

#[async_trait]
impl KeywordOptimizer for FirstLetterExtractor {
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
