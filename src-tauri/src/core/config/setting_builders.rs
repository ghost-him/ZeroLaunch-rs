//! SettingDefinition 构建器函数。
//!
//! 提供简洁的工厂函数来创建常见的 SettingDefinition，避免在各组件中重复手写 FieldDefinition 结构体。
//! 这些函数覆盖 90% 的简单字段场景。
//! 复杂嵌套结构（如 Array + Object items、带 config_action 的字段）建议直接手写。

use crate::core::types::setting_def::{FieldDefinition, SettingDefinition, SettingType};
use serde_json::json;

#[allow(clippy::too_many_arguments)]
pub fn num_field(
    key: &str,
    label: &str,
    desc: &str,
    group: &str,
    order: u32,
    default: f64,
    min: f64,
    max: f64,
    step: f64,
) -> SettingDefinition {
    SettingDefinition {
        field: FieldDefinition {
            key: key.to_string(),
            label: label.to_string(),
            description: desc.to_string(),
            setting_type: SettingType::Number {
                min: Some(min),
                max: Some(max),
                step: Some(step),
            },
            default_value: json!(default),
            visible: true,
            editable: true,
        },
        group: Some(group.to_string()),
        order,
        config_action: None,
    }
}

pub fn text_field(
    key: &str,
    label: &str,
    desc: &str,
    group: &str,
    order: u32,
    default: &str,
) -> SettingDefinition {
    SettingDefinition {
        field: FieldDefinition {
            key: key.to_string(),
            label: label.to_string(),
            description: desc.to_string(),
            setting_type: SettingType::Text,
            default_value: json!(default),
            visible: true,
            editable: true,
        },
        group: Some(group.to_string()),
        order,
        config_action: None,
    }
}

pub fn bool_field(
    key: &str,
    label: &str,
    desc: &str,
    group: &str,
    order: u32,
    default: bool,
) -> SettingDefinition {
    SettingDefinition {
        field: FieldDefinition {
            key: key.to_string(),
            label: label.to_string(),
            description: desc.to_string(),
            setting_type: SettingType::Boolean,
            default_value: json!(default),
            visible: true,
            editable: true,
        },
        group: Some(group.to_string()),
        order,
        config_action: None,
    }
}

pub fn color_field(
    key: &str,
    label: &str,
    desc: &str,
    group: &str,
    order: u32,
    default: &str,
) -> SettingDefinition {
    SettingDefinition {
        field: FieldDefinition {
            key: key.to_string(),
            label: label.to_string(),
            description: desc.to_string(),
            setting_type: SettingType::Color,
            default_value: json!(default),
            visible: true,
            editable: true,
        },
        group: Some(group.to_string()),
        order,
        config_action: None,
    }
}

pub fn select_field(
    key: &str,
    label: &str,
    desc: &str,
    group: &str,
    order: u32,
    options: Vec<&str>,
    default: &str,
) -> SettingDefinition {
    SettingDefinition {
        field: FieldDefinition {
            key: key.to_string(),
            label: label.to_string(),
            description: desc.to_string(),
            setting_type: SettingType::Select {
                options: options.iter().map(|s| s.to_string()).collect(),
            },
            default_value: json!(default),
            visible: true,
            editable: true,
        },
        group: Some(group.to_string()),
        order,
        config_action: None,
    }
}
