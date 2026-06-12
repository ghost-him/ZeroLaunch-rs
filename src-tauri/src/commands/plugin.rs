//! 插件管理 IPC 命令（第三方插件）。
//!
//! 所有业务逻辑委托给 PluginManager。
//! 此文件为薄代理层：提取参数 → 调用 PluginManager → 返回结果。

use crate::core::types::BridgeError;
use crate::state::app_state::AppState;
use std::sync::Arc;
use tauri::State;
use zerolaunch_plugin_host::manager::InstalledPluginInfo;
use zerolaunch_plugin_protocol::Manifest;

// ── Commands ─────────────────────────────────────────────────────

/// List all installed third-party plugins.
#[tauri::command]
pub async fn plugin_list(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<InstalledPluginInfo>, BridgeError> {
    let plugin_manager = state.get_plugin_manager();
    Ok(plugin_manager.list_third_party_details())
}

/// 获取第三方插件的完整 manifest。
#[tauri::command]
pub async fn plugin_get_manifest(
    plugin_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Manifest, BridgeError> {
    let plugin_manager = state.get_plugin_manager();
    plugin_manager
        .get_manifest(&plugin_id)
        .ok_or_else(|| BridgeError::not_found(&plugin_id))
}

/// Install a plugin from a local .zip file or directory.
/// Emits `plugin-installed` on success.
#[tauri::command]
pub async fn plugin_install_local(
    file_path: String,
    state: State<'_, Arc<AppState>>,
) -> Result<InstalledPluginInfo, BridgeError> {
    let plugin_manager = state.get_plugin_manager();
    let path = std::path::PathBuf::from(&file_path);
    let app_handle = state.get_main_handle();

    plugin_manager
        .install(&path, app_handle)
        .await
        .map_err(BridgeError::internal)
}

/// Reload a third-party plugin.
/// Emits `plugin-installed` on success.
#[tauri::command]
pub async fn plugin_reload(
    plugin_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), BridgeError> {
    let plugin_manager = state.get_plugin_manager();
    let app_handle = state.get_main_handle();

    plugin_manager
        .reload(&plugin_id, app_handle)
        .await
        .map_err(BridgeError::internal)
}

/// Uninstall a third-party plugin.
/// Emits `plugin-uninstalled` on success.
#[tauri::command]
pub async fn plugin_uninstall(
    plugin_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), BridgeError> {
    let plugin_manager = state.get_plugin_manager();
    let app_handle = state.get_main_handle();

    plugin_manager
        .uninstall(&plugin_id, app_handle)
        .await
        .map_err(BridgeError::internal)
}

/// Enable or disable all components of a plugin (third-party or builtin).
#[tauri::command]
pub async fn plugin_set_enabled(
    plugin_id: String,
    enabled: bool,
    state: State<'_, Arc<AppState>>,
) -> Result<(), BridgeError> {
    let plugin_manager = state.get_plugin_manager();
    plugin_manager
        .set_enabled(&plugin_id, enabled)
        .map_err(BridgeError::internal)
}

/// 获取插件 stderr 日志的最近 N 行。
#[tauri::command]
pub async fn plugin_get_logs(
    plugin_id: String,
    tail_lines: Option<usize>,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<String>, BridgeError> {
    let plugin_manager = state.get_plugin_manager();
    plugin_manager
        .get_logs(&plugin_id, tail_lines.unwrap_or(50))
        .map_err(BridgeError::internal)
}
