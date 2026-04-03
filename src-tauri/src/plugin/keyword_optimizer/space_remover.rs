use crate::plugin_system::types::FieldDefinition;
use crate::plugin_system::{
    types::{ComponentType, ConfigError, Configurable, KeywordOptimizer, SettingDefinition},
    SettingType,
};
pub struct SpaceRemover {
    priority: i32,
    uses_context: bool,
}

impl Default for SpaceRemover {
    fn default() -> Self {
        Self::new()
    }
}

impl SpaceRemover {
    pub fn new() -> Self {
        Self {
            priority: 60,
            uses_context: true,
        }
    }

    fn remove_string_space(&self, input_text: &str) -> String {
        input_text.chars().filter(|&c| c != ' ').collect()
    }
}

impl Configurable for SpaceRemover {
    fn component_id(&self) -> &str {
        "space-remover"
    }

    fn component_name(&self) -> &str {
        "空格移除器"
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
                    default_value: serde_json::json!(60),
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

impl KeywordOptimizer for SpaceRemover {
    fn optimize(&self, keyword: &str) -> Vec<String> {
        let result = self.remove_string_space(keyword);
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
