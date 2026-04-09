use crate::plugin_system::types::FieldDefinition;
use crate::plugin_system::{
    types::{ComponentType, ConfigError, Configurable, KeywordOptimizer, SettingDefinition},
    SettingType,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 拼音数据条目
#[derive(Serialize, Deserialize, Debug)]
struct PinyinItem {
    pinyin: String,
    word: String,
}

/// 拼音转换器，将中文字符转换为对应的拼音
///
/// 实现为 KeywordOptimizer 插件，在 CandidatePipeline 中按优先级执行，
/// 将候选项名称中的中文字符转换为拼音，以支持拼音搜索和首字母搜索。
pub struct PinyinConverter {
    /// 汉字到拼音的映射表，使用 char 作为 key 避免堆分配
    pinyin: HashMap<char, String>,
    /// 优化器执行优先级，数值越小越先执行
    priority: i32,
    /// 是否对所有已累积的关键词进行优化
    uses_context: bool,
}

impl Default for PinyinConverter {
    fn default() -> Self {
        Self::new()
    }
}

impl PinyinConverter {
    const PINYIN_DATA: &'static str = include_str!("./pinyin.json");

    /// 创建拼音转换器实例
    pub fn new() -> Self {
        let items: Vec<PinyinItem> =
            serde_json::from_str(Self::PINYIN_DATA).expect("Failed to parse pinyin data");

        let mut char_to_pinyin: HashMap<char, String> = HashMap::new();
        for item in items {
            // word 字段为单汉字，取第一个字符作为 key
            if let Some(ch) = item.word.chars().next() {
                char_to_pinyin.insert(ch, item.pinyin);
            }
        }

        PinyinConverter {
            pinyin: char_to_pinyin,
            priority: 25,
            uses_context: true,
        }
    }

    /// 将输入字符串中的汉字转换为拼音
    ///
    /// 转换规则：
    /// - 每个汉字替换为对应的拼音，拼音后附加一个空格
    /// - 连续的汉字之间不加额外空格（拼音后自带空格已足够分隔）
    /// - ASCII 字符原样保留
    /// - 汉字与 ASCII 字符之间插入一个空格用于分隔
    fn convert_to_pinyin(&self, input: &str) -> String {
        let mut result = String::new();
        let mut prev_is_han = false;

        for c in input.chars() {
            if let Some(pinyin) = self.pinyin.get(&c) {
                // 如果前一个字符不是汉字，且结果字符串不为空，插入一个空格分隔
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
            },
        ]
    }

    fn get_settings(&self) -> serde_json::Value {
        serde_json::json!({
            "priority": self.priority,
            "uses_context": self.uses_context
        })
    }

    fn apply_settings(&mut self, settings: serde_json::Value) -> Result<(), ConfigError> {
        if let Some(priority) = settings.get("priority").and_then(|v| v.as_f64()) {
            self.priority = priority as i32;
        }
        if let Some(uses_context) = settings.get("uses_context").and_then(|v| v.as_bool()) {
            self.uses_context = uses_context;
        }
        Ok(())
    }
}

impl KeywordOptimizer for PinyinConverter {
    /// 将关键词中的中文字符转换为拼音
    fn optimize(&self, keyword: &str) -> Vec<String> {
        let result = self.convert_to_pinyin(keyword);
        vec![result]
    }

    fn uses_context(&self) -> bool {
        self.uses_context
    }

    fn get_priority(&self) -> i32 {
        self.priority
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
