use serde::{Deserialize, Serialize};
use zerolaunch_plugin_api::config::{ComponentType, SettingsContribution};

/// 组件概览条目 —— 配置管理器向 IPC / CLI 提供的组件清单（跨 IPC 序列化）。
///
/// 由 `ConfigManager::get_all_components` 组装，命令层与本地 HTTP API 直接下发；
/// 字段即下发形状，不再有第二个 DTO。
#[derive(Debug, Clone, Serialize)]
pub struct ComponentInfo {
    /// 组件 id。序列化键名 `componentId`。
    #[serde(rename = "componentId")]
    pub component_id: String,
    /// 组件显示名（i18n key）。序列化键名 `componentName`。
    #[serde(rename = "componentName")]
    pub component_name: String,
    /// 组件说明（i18n key）。序列化键名 `componentDescription`。
    #[serde(rename = "componentDescription")]
    pub component_description: String,
    /// 组件类别。序列化键名 `componentType`。
    #[serde(rename = "componentType")]
    pub component_type: ComponentType,
    /// 排序优先级（越小越靠前）。序列化键名 `priority`。
    #[serde(rename = "priority")]
    pub priority: u32,
    /// 当前是否启用。序列化键名 `enabled`。
    #[serde(rename = "enabled")]
    pub enabled: bool,
    /// 出厂默认是否启用。序列化键名 `defaultEnabled`。
    #[serde(rename = "defaultEnabled")]
    pub default_enabled: bool,
}

/// 组件配置 schema —— 配置管理器向 IPC / CLI 提供的单个组件 schema（跨 IPC 序列化）。
///
/// 由 `ConfigManager::get_component_schema` 组装，命令层与本地 HTTP API 直接下发。
#[derive(Debug, Clone, Serialize)]
pub struct ComponentSchema {
    /// 组件 id。序列化键名 `componentId`。
    #[serde(rename = "componentId")]
    pub component_id: String,
    /// 组件显示名（i18n key）。序列化键名 `componentName`。
    #[serde(rename = "componentName")]
    pub component_name: String,
    /// 组件说明（i18n key）。序列化键名 `componentDescription`。
    #[serde(rename = "componentDescription")]
    pub component_description: String,
    /// 组件类别。序列化键名 `componentType`。
    #[serde(rename = "componentType")]
    pub component_type: ComponentType,
    /// schema 主体（字段定义与 UI 元数据）。序列化键名 `contribution`。
    #[serde(rename = "contribution")]
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
