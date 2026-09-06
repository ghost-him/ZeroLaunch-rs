use crate::core::config::setting_builders::SchemaBuilder;
use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use zerolaunch_plugin_api::config::{
    ComponentCore, ComponentType, ConfigError, Configurable, SettingDefinition,
};
use zerolaunch_plugin_api::{KeywordInputSource, KeywordOptimizer};

/// Default priority value for FirstLetterExtractorSettings.
fn default_priority_50() -> u32 {
    50
}

/// 首字母提取器的可持久化配置
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FirstLetterExtractorSettings {
    #[serde(rename = "priority", default = "default_priority_50")]
    priority: u32,
}

impl FirstLetterExtractorSettings {
    fn new() -> Self {
        Self { priority: 50 }
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
        vec![SchemaBuilder::number(
            "priority",
            t_key!("first-letter-extractor", "fields.priority.label"),
            t_key!("first-letter-extractor", "fields.priority.desc"),
        )
        .order(0)
        .default(50.0)
        .min(1.0)
        .max(100.0)
        .step(1.0)
        .build()]
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
        // 首字母缩写消费拼音转换器的登记输出（QQ音乐→qq yin le→qyl、VSC→vsc）；
        // 不直接吃归一化基/派生词，避免对单 token 取首字符产生无意义单字符（vsc→v、qq→q）。
        // pinyin-converter 是普通生产者（priority 25 < 本器 50，先于本器执行）。
        KeywordInputSource::OptimizerOutput {
            producer_id: "pinyin-converter".to_string(),
        }
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
