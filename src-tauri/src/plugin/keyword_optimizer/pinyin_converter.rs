use crate::plugin_system::types::FieldDefinition;
use crate::plugin_system::{
    types::{ComponentType, ConfigError, Configurable, KeywordOptimizer, SettingDefinition},
    SettingType,
};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
struct PinyinItem {
    pinyin: String,
    word: String,
}

struct PinyinConverterInner {
    priority: i32,
    uses_context: bool,
    pinyin: HashMap<char, String>,
}

impl PinyinConverterInner {
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
            uses_context: true,
            pinyin: char_to_pinyin,
        }
    }

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

    fn optimize(&self, keyword: &str) -> Vec<String> {
        let result = self.convert_to_pinyin(keyword);
        vec![result]
    }
}

pub struct PinyinConverter {
    inner: RwLock<PinyinConverterInner>,
}

impl Default for PinyinConverter {
    fn default() -> Self {
        Self::new()
    }
}

impl PinyinConverter {
    pub fn new() -> Self {
        PinyinConverter {
            inner: RwLock::new(PinyinConverterInner::new()),
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

    fn component_type(&self) -> ComponentType {
        ComponentType::KeywordOptimizer
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![
            SettingDefinition {
                field: FieldDefinition {
                    key: "priority".to_string(),
                    label: "优先级".to_string(),
                    description: "优化器执行优先级，数值越小越先执行".to_string(),
                    setting_type: SettingType::Number {
                        min: Some(1.0),
                        max: Some(100.0),
                        step: Some(1.0),
                    },
                    default_value: serde_json::json!(25),
                    visible: true,
                    editable: true,
                },
                group: None,
                order: 0,
                config_action: None,
            },
            SettingDefinition {
                field: FieldDefinition {
                    key: "uses_context".to_string(),
                    label: "上下文优化".to_string(),
                    description: "是否对所有已累积的关键词进行拼音转换".to_string(),
                    setting_type: SettingType::Boolean,
                    default_value: serde_json::json!(true),
                    visible: true,
                    editable: true,
                },
                group: None,
                order: 1,
                config_action: None,
            },
        ]
    }

    fn get_settings(&self) -> serde_json::Value {
        let inner = self.inner.read();
        serde_json::json!({
            "priority": inner.priority,
            "uses_context": inner.uses_context
        })
    }

    fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        let mut inner = self.inner.write();
        if let Some(priority) = settings.get("priority").and_then(|v| v.as_f64()) {
            inner.priority = priority as i32;
        }
        if let Some(uses_context) = settings.get("uses_context").and_then(|v| v.as_bool()) {
            inner.uses_context = uses_context;
        }
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
