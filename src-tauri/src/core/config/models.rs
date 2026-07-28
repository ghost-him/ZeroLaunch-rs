use serde::{Deserialize, Serialize};
use zerolaunch_plugin_api::config::{ComponentType, SettingsContribution};

/// 配置管理器向 IPC 边界提供的组件概览快照，不携带序列化职责。
#[derive(Debug, Clone)]
pub struct ComponentInfoSnapshot {
    pub component_id: String,
    pub component_name: String,
    pub component_description: String,
    pub component_type: ComponentType,
    pub priority: u32,
    pub enabled: bool,
    pub default_enabled: bool,
}

/// 配置管理器向 IPC 边界提供的组件 schema 快照，不携带序列化职责。
#[derive(Debug, Clone)]
pub struct ComponentSchemaSnapshot {
    pub component_id: String,
    pub component_name: String,
    pub component_description: String,
    pub component_type: ComponentType,
    pub contribution: SettingsContribution,
}

/// 持久化配置文件格式（config_v3.json）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistentConfig {
    /// 配置格式版本
    #[serde(rename = "version", default = "default_config_version")]
    pub version: String,
    /// 各组件的持久化状态
    #[serde(rename = "components", default)]
    pub components: std::collections::HashMap<String, ComponentPersistentState>,
}

fn default_config_version() -> String {
    "3".to_string()
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
    #[serde(rename = "enabled", default)]
    pub enabled: bool,
    /// 配置值
    #[serde(rename = "settings", default)]
    pub settings: serde_json::Value,
}
