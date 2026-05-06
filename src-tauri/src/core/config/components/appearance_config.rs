use crate::core::types::setting_def::{FieldDefinition, SettingDefinition, SettingType};
use crate::core::types::{ComponentType, ConfigError, Configurable};
use parking_lot::RwLock;
use serde_json::json;

/// 外观配置组件。
/// 管理主题（浅色/深色/跟随系统）和语言偏好。
pub struct AppearanceConfigComponent {
    settings: RwLock<serde_json::Value>,
}

impl Default for AppearanceConfigComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl AppearanceConfigComponent {
    pub fn new() -> Self {
        Self {
            settings: RwLock::new(Self::default_settings_value()),
        }
    }

    fn default_settings_value() -> serde_json::Value {
        json!({
            "theme": "system",
            "language": "zh-Hans"
        })
    }
}

impl Configurable for AppearanceConfigComponent {
    fn component_id(&self) -> &str {
        "appearance"
    }

    fn component_name(&self) -> &str {
        "外观"
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::Core
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![
            SettingDefinition {
                field: FieldDefinition {
                    key: "theme".to_string(),
                    label: "主题".to_string(),
                    description: "选择浅色、深色或跟随系统主题".to_string(),
                    setting_type: SettingType::Select {
                        options: vec![
                            "system".to_string(),
                            "light".to_string(),
                            "dark".to_string(),
                        ],
                    },
                    default_value: json!("system"),
                    visible: true,
                    editable: true,
                },
                group: None,
                order: 0,
                config_action: None,
            },
            SettingDefinition {
                field: FieldDefinition {
                    key: "language".to_string(),
                    label: "语言".to_string(),
                    description: "界面显示语言".to_string(),
                    setting_type: SettingType::Select {
                        options: vec!["zh-Hans".to_string(), "en".to_string()],
                    },
                    default_value: json!("zh-Hans"),
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
        self.settings.read().clone()
    }

    fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        // 校验 theme 值
        if let Some(theme) = settings.get("theme").and_then(|v| v.as_str()) {
            if !["system", "light", "dark"].contains(&theme) {
                return Err(ConfigError::ValidationFailed(format!(
                    "Invalid theme value: {}",
                    theme
                )));
            }
        }
        // 校验 language 值
        if let Some(lang) = settings.get("language").and_then(|v| v.as_str()) {
            if !["zh-Hans", "en"].contains(&lang) {
                return Err(ConfigError::ValidationFailed(format!(
                    "Invalid language value: {}",
                    lang
                )));
            }
        }
        *self.settings.write() = settings;
        Ok(())
    }

    fn get_default_settings(&self) -> serde_json::Value {
        Self::default_settings_value()
    }

    fn default_enabled(&self) -> bool {
        true
    }
}
