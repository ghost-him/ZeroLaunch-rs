//! 插件展示条目 —— 插件管理页 / CLI 的插件级视图。
//!
//! 字段权威是 `PluginMetadata`（第三方取自清单，内置由代码声明）；本类型只追加
//! 运行态三件事：进程状态、启用态、已注册组件 id。构造必须经 `from_metadata`，
//! 不再允许各处自行拼装，避免同一行数据出现多份字段来源。

use serde::Serialize;
use zerolaunch_plugin_api::plugin::{PluginKind, PluginMetadata, PluginMode};

use crate::manager::PluginRuntimeState;

/// 插件列表 / 详情共用的插件级条目（跨 IPC 序列化，键名为 camelCase）。
#[derive(Debug, Clone, Serialize)]
pub struct PluginInfo {
    /// 插件 id。序列化键名 `pluginId`。
    #[serde(rename = "pluginId")]
    pub plugin_id: String,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "version")]
    pub version: String,
    #[serde(rename = "description")]
    pub description: String,
    #[serde(rename = "author")]
    pub author: String,
    /// 运行状态：已加载取进程状态，未加载固定为 error。
    #[serde(rename = "state")]
    pub state: PluginRuntimeState,
    /// 是否启用：插件自身 Plugin 组件的启用态；未加载恒为 false。
    #[serde(rename = "enabled")]
    pub enabled: bool,
    /// 插件种类：内置或第三方。
    #[serde(rename = "kind", default)]
    pub kind: PluginKind,
    #[serde(rename = "priority")]
    pub priority: u32,
    /// 可卸载（仅磁盘安装的第三方插件）。
    #[serde(rename = "canUninstall")]
    pub can_uninstall: bool,
    /// 可重载（仅磁盘安装的第三方插件）。
    #[serde(rename = "canReload")]
    pub can_reload: bool,
    /// 可查看进程日志（仅第三方插件有子进程）。
    #[serde(rename = "canViewLogs")]
    pub can_view_logs: bool,
    /// 可切换启用态（未加载的第三方插件无组件可写）。
    #[serde(rename = "canToggle")]
    pub can_toggle: bool,
    /// 全局唤醒快捷键（如 "Ctrl+E"），可空。
    #[serde(rename = "hotkey", default)]
    pub hotkey: Option<String>,
    /// 插件显示图标（data URL），可空表示无图标。
    #[serde(rename = "icon", default)]
    pub icon: Option<String>,
    /// 插件形态：inline = 行内插件；panel = 独立插件（唯一权威依据）。
    #[serde(rename = "mode", default)]
    pub mode: PluginMode,
}

impl PluginInfo {
    /// 由插件元数据投影为展示条目。
    /// 参数：meta - 插件级元数据（字段唯一来源）；state - 进程状态；
    /// enabled - 插件级启用态（Plugin 组件的启用态；其 id 恒等于插件 id）。
    /// 返回：IPC 展示条目；能力位由 kind/state 在此统一派生。
    pub fn from_metadata(meta: &PluginMetadata, state: PluginRuntimeState, enabled: bool) -> Self {
        let third_party = meta.kind == PluginKind::ThirdParty;
        let can_toggle = !(third_party && matches!(state, PluginRuntimeState::Error));
        Self {
            plugin_id: meta.id.clone(),
            name: meta.name.clone(),
            version: meta.version.clone(),
            description: meta.description.clone(),
            author: meta.author.clone(),
            state,
            enabled,
            kind: meta.kind,
            priority: meta.priority,
            can_uninstall: third_party,
            can_reload: third_party,
            can_view_logs: third_party,
            can_toggle,
            hotkey: meta.hotkey.clone(),
            icon: meta.icon.clone(),
            mode: meta.mode,
        }
    }
}
