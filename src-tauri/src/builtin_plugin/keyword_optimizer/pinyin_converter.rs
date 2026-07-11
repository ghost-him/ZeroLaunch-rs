use crate::core::config::setting_builders::SchemaBuilder;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use zerolaunch_plugin_api::config::{ComponentType, ConfigError, Configurable, SettingDefinition};
use zerolaunch_plugin_api::KeywordOptimizer;

#[derive(Serialize, Deserialize, Debug)]
struct PinyinItem {
    pinyin: String,
    word: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PinyinConverterSettings {
    #[serde(rename = "priority", default = "default_priority_25")]
    priority: i32,
    #[serde(rename = "uses_context", default = "default_uses_context_false")]
    uses_context: bool,
    /// 预加载的汉字到拼音映射表，不属于用户配置，序列化时跳过。
    #[serde(skip)]
    pinyin: HashMap<char, String>,
}

fn default_priority_25() -> i32 {
    25
}

fn default_uses_context_false() -> bool {
    false
}

impl Default for PinyinConverterSettings {
    fn default() -> Self {
        Self::new()
    }
}

impl PinyinConverterSettings {
    const PINYIN_DATA: &'static str = include_str!("./pinyin.json");

    fn new() -> Self {
        let items: Vec<PinyinItem> =
            serde_json::from_str(Self::PINYIN_DATA).expect("Failed to parse pinyin data");

        let mut char_to_pinyin: HashMap<char, String> = HashMap::new();
        for item in items {
            if let Some(ch) = item.word.chars().next() {
                char_to_pinyin.insert(ch, item.pinyin);
            }
        }

        Self {
            priority: 25,
            uses_context: false,
            pinyin: char_to_pinyin,
        }
    }

    /// 将输入字符串中的汉字转换为拼音。
    /// 连续汉字用空格分隔，非汉字原样保留。
    fn convert_to_pinyin(&self, input: &str) -> String {
        let mut result = String::new();
        let mut prev_is_han = false;

        for c in input.chars() {
            if let Some(pinyin) = self.pinyin.get(&c) {
                if !prev_is_han && !result.is_empty() {
                    result.push(' ');
                }
                result.push_str(pinyin);
                result.push(' ');
                prev_is_han = true;
            } else {
                result.push(c);
                prev_is_han = false;
            }
        }

        result.trim_end().to_string()
    }

    /// 对关键词执行拼音转换优化。
    fn optimize(&self, keyword: &str) -> Vec<String> {
        let result = self.convert_to_pinyin(keyword);
        vec![result]
    }
}

pub struct PinyinConverter {
    inner: RwLock<PinyinConverterSettings>,
}

impl Default for PinyinConverter {
    fn default() -> Self {
        Self::new()
    }
}

impl PinyinConverter {
    pub fn new() -> Self {
        PinyinConverter {
            inner: RwLock::new(PinyinConverterSettings::new()),
        }
    }
}

impl Configurable for PinyinConverter {
    fn component_id(&self) -> &str {
        "pinyin-converter"
    }

    fn component_name(&self) -> &str {
        "拼音转换器"
    }

    fn component_description(&self) -> &str {
        "将中文关键词转换为拼音以支持拼音搜索"
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::KeywordOptimizer
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![
            SchemaBuilder::number("priority", "优先级", "优化器执行优先级，数值越小越先执行")
                .order(0)
                .default(25.0)
                .min(1.0)
                .max(100.0)
                .step(1.0)
                .build(),
            SchemaBuilder::boolean(
                "uses_context",
                "上下文优化",
                "是否对所有已累积的关键词进行拼音转换",
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
        let mut parsed: PinyinConverterSettings =
            serde_json::from_value(settings).unwrap_or_default();
        // 保留预加载的拼音字典，该字段不属于用户配置
        parsed.pinyin = self.inner.read().pinyin.clone();
        *self.inner.write() = parsed;
        Ok(())
    }
}

impl KeywordOptimizer for PinyinConverter {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_chinese_only() {
        let converter = PinyinConverter::new();
        let result = converter.optimize("微信");
        assert_eq!(result, vec!["wei xin"]);
    }

    #[test]
    fn test_convert_mixed() {
        let converter = PinyinConverter::new();
        let result = converter.optimize("微信WeChat");
        assert_eq!(result, vec!["wei xin WeChat"]);
    }

    #[test]
    fn test_convert_ascii_only() {
        let converter = PinyinConverter::new();
        let result = converter.optimize("chrome");
        assert_eq!(result, vec!["chrome"]);
    }
}

use crate::plugin_framework::builtin_registry::KeywordOptimizerEntry;
use std::sync::Arc;

pub(crate) fn build_pinyin_converter() -> (Arc<dyn Configurable>, Arc<dyn KeywordOptimizer>) {
    let opt: Arc<dyn KeywordOptimizer> = Arc::new(PinyinConverter::new());
    let configurable: Arc<dyn Configurable> = opt.clone();
    (configurable, opt)
}

::inventory::submit! {
    KeywordOptimizerEntry {
        component_id: "pinyin-converter",
        priority: 50,
        factory: build_pinyin_converter,
    }
}
