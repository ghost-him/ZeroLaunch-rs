//! 插件管理 IPC 命令（第三方插件）。
//!
//! 业务协调逻辑：组合 PluginManager（生命周期）与 ConfigManager（配置），
//! 不新增中间层，直接在命令处理器中编排两个管理器的调用。

use crate::core::BridgeError;
use crate::state::app_state::AppState;
use std::sync::Arc;
use tauri::State;
use zerolaunch_plugin_api::config::Configurable;
use zerolaunch_plugin_host::manager::InstalledPluginInfo;
use zerolaunch_plugin_protocol::Manifest;

// ── Commands ─────────────────────────────────────────────────────

/// List all installed third-party plugins with runtime details.
#[tauri::command]
pub async fn plugin_list(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<InstalledPluginInfo>, BridgeError> {
    let pm = state.get_plugin_manager();
    let cm = state.get_config_manager();
    let hm = pm.host_manager();

    Ok(hm.list_plugin_info(|a| {
        a.configurables
            .iter()
            .all(|c| cm.is_enabled(c.component_id()))
            && !a.configurables.is_empty()
    }))
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
///
/// 第三方插件：遍历其所有 Configurable 逐个调用 CM.set_enabled()。
/// 内置组件：直接按 plugin_id 调用 CM.set_enabled()。
#[tauri::command]
pub async fn plugin_set_enabled(
    plugin_id: String,
    enabled: bool,
    state: State<'_, Arc<AppState>>,
) -> Result<(), BridgeError> {
    let pm = state.get_plugin_manager();
    let cm = state.get_config_manager();
    let hm = pm.host_manager();

    // 第三方插件：为每个组件设置 enabled
    if let Some(adapters) = hm.adapters.get(&plugin_id) {
        for c in &adapters.configurables {
            cm.set_enabled(c.component_id(), enabled)
                .map_err(|e| BridgeError::internal(e.to_string()))?;
        }
        return Ok(());
    }

    // 内置组件：直接按 ID 设置
    cm.set_enabled(&plugin_id, enabled)
        .map_err(|e| BridgeError::internal(e.to_string()))
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
