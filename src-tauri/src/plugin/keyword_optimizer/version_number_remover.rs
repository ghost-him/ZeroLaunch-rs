use crate::plugin_system::types::FieldDefinition;
use crate::plugin_system::{
    types::{ComponentType, ConfigError, Configurable, KeywordOptimizer, SettingDefinition},
    SettingType,
};
pub struct VersionNumberRemover {
    priority: i32,
    uses_context: bool,
}

impl Default for VersionNumberRemover {
    fn default() -> Self {
        Self::new()
    }
}

impl VersionNumberRemover {
    pub fn new() -> Self {
        Self {
            priority: 10,
            uses_context: false,
        }
    }

    fn remove_version_number(&self, input_text: &str) -> String {
        let mut ret = String::new();
        let mut s = 0;
        let mut in_version = false;
        let chars: Vec<char> = input_text.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            let ch = chars[i];

            if ch == '(' {
                s += 1;
                in_version = true;
            } else if ch == ')' {
                if s > 0 {
                    s -= 1;
                }
                in_version = false;
            } else if s == 0 && !in_version {
                if (ch.is_ascii_digit() || ch == '.') && i > 0 && chars[i - 1] == ' ' {
                    while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                        i += 1;
                    }
                    while i < chars.len() && chars[i] == ' ' {
                        i += 1;
                    }
                    i = i.saturating_sub(1);
                    i += 1;
                    continue;
                }
                ret.push(ch);
            }

            i += 1;
        }

        while ret.ends_with(' ') {
            ret.pop();
        }

        ret
    }
}

impl Configurable for VersionNumberRemover {
    fn component_id(&self) -> &str {
        "version-number-remover"
    }

    fn component_name(&self) -> &str {
        "版本号移除器"
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
                    default_value: serde_json::json!(10),
                    visible: true,
                    editable: true,
                },
                group: None,
                order: 0,
            },
            SettingDefinition {
                field: FieldDefinition {
                    key: "uses_context".to_string(),
                    label: "使用上下文".to_string(),
                    description: "是否对所有已累积的关键词进行优化，而非仅对原始名称优化"
                        .to_string(),
                    setting_type: SettingType::Boolean,
                    default_value: serde_json::json!(false),
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

impl KeywordOptimizer for VersionNumberRemover {
    fn optimize(&self, keyword: &str) -> Vec<String> {
        let result = self.remove_version_number(keyword);
        if result.is_empty() || result == keyword {
            Vec::new()
        } else {
            vec![result]
        }
    }

    fn uses_context(&self) -> bool {
        self.uses_context
    }

    fn get_priority(&self) -> i32 {
        self.priority
    }
}
