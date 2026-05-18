use serde::{Deserialize, Serialize};

use crate::core::types::{ComponentType, SettingDefinition};

/// 组件概览信息，用于前端展示组件列表
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentInfo {
    /// 组件唯一标识
    #[serde(rename = "componentId")]
    pub component_id: String,
    /// 组件显示名称
    #[serde(rename = "componentName")]
    pub component_name: String,
    /// 组件类型
    #[serde(rename = "componentType")]
    pub component_type: ComponentType,
    /// 组件是否启用
    #[serde(rename = "enabled")]
    pub enabled: bool,
    /// 组件默认是否启用
    #[serde(rename = "defaultEnabled")]
    pub default_enabled: bool,
}

/// 组件配置 Schema，用于前端渲染配置表单
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentSchema {
    /// 组件唯一标识
    #[serde(rename = "componentId")]
    pub component_id: String,
    /// 组件显示名称
    #[serde(rename = "componentName")]
    pub component_name: String,
    /// 组件类型
    #[serde(rename = "componentType")]
    pub component_type: ComponentType,
    /// 配置项定义列表
    #[serde(rename = "settings")]
    pub settings: Vec<SettingDefinition>,
}

/// 持久化配置文件格式（config_v3.json）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistentConfig {
    /// 配置格式版本
    #[serde(rename = "version")]
    pub version: String,
    /// 各组件的持久化状态
    #[serde(rename = "components")]
    pub components: std::collections::HashMap<String, ComponentPersistentState>,
}

impl Default for PersistentConfig {
    fn default() -> Self {
        Self {
            version: "3".to_string(),
            components: std::collections::HashMap::new(),
        }
    }
}

/// 单个组件的持久化状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentPersistentState {
    /// 是否启用
    #[serde(rename = "enabled")]
    pub enabled: bool,
    /// 配置值
    #[serde(rename = "settings")]
    pub settings: serde_json::Value,
}
