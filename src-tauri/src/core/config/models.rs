use serde::{Deserialize, Serialize};

use crate::core::types::{ComponentType, SettingDefinition};

/// 组件概览信息，用于前端展示组件列表
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentInfo {
    /// 组件唯一标识
    pub component_id: String,
    /// 组件显示名称
    pub component_name: String,
    /// 组件类型
    pub component_type: ComponentType,
    /// 组件是否启用
    pub enabled: bool,
    /// 组件默认是否启用
    pub default_enabled: bool,
}

/// 组件配置 Schema，用于前端渲染配置表单
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentSchema {
    /// 组件唯一标识
    pub component_id: String,
    /// 组件显示名称
    pub component_name: String,
    /// 组件类型
    pub component_type: ComponentType,
    /// 配置项定义列表
    pub settings: Vec<SettingDefinition>,
}

/// 持久化配置文件格式（config_v3.json）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistentConfig {
    /// 配置格式版本
    pub version: String,
    /// 各组件的持久化状态
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
#[serde(rename_all = "camelCase")]
pub struct ComponentPersistentState {
    /// 是否启用
    pub enabled: bool,
    /// 配置值
    pub settings: serde_json::Value,
}
