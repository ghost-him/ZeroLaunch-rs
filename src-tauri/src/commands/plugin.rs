//! Plugin management IPC commands (third-party plugins).
//!
//! All business logic is delegated to PluginManager.
//! This file is a thin proxy: extract args → call PluginManager → return result.

use crate::core::types::BridgeError;
use crate::plugin_manager::manager::PluginManager;
use crate::state::app_state::AppState;
use std::sync::Arc;
use tauri::State;
use zerolaunch_plugin_host::manager::{InstalledPluginInfo, PluginHostManager};

/// Resolve the host manager from AppState.
fn get_host_manager(
    state: &State<'_, Arc<AppState>>,
) -> Result<Arc<PluginHostManager>, BridgeError> {
    state
        .get_plugin_host_manager()
        .ok_or_else(|| BridgeError::internal("PluginHostManager not initialized"))
}

/// Resolve the plugin manager from AppState.
fn get_plugin_manager(state: &State<'_, Arc<AppState>>) -> Arc<PluginManager> {
    state.get_plugin_manager()
}

// ── Commands ─────────────────────────────────────────────────────

/// List all installed third-party plugins.
#[tauri::command]
pub async fn plugin_list(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<InstalledPluginInfo>, BridgeError> {
    let plugin_manager = get_plugin_manager(&state);
    Ok(plugin_manager.list_third_party_details())
}

/// Get the full manifest of a third-party plugin.
#[tauri::command]
pub async fn plugin_get_manifest(
    plugin_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, BridgeError> {
    let host_manager = get_host_manager(&state)?;

    let adapters = host_manager
        .adapters
        .get(&plugin_id)
        .ok_or_else(|| BridgeError::not_found(&plugin_id))?;

    let manifest = &adapters.manifest;
    let json = serde_json::json!({
        "pluginId": manifest.plugin.id,
        "name": manifest.plugin.name,
        "version": manifest.plugin.version,
        "description": manifest.plugin.description,
        "author": manifest.plugin.author,
        "homepage": manifest.plugin.homepage,
        "minHostVersion": manifest.plugin.min_host_version,
        "runtime": {
            "command": manifest.runtime.command,
            "args": manifest.runtime.args,
            "startupTimeout": manifest.runtime.startup_timeout,
            "autoRestart": manifest.runtime.auto_restart,
            "maxRestart": manifest.runtime.max_restart,
        },
        "components": {
            "provides": manifest.components.provides,
        },
        "ui": manifest.ui.as_ref().map(|ui| {
            serde_json::json!({
                "panelEntry": ui.panel_entry,
                "settingsEntry": ui.settings_entry,
                "resultItemEntry": ui.result_item_entry,
            })
        }),
    });

    Ok(json)
}

/// Install a plugin from a local .zip file or directory.
/// Emits `plugin-installed` on success.
#[tauri::command]
pub async fn plugin_install_local(
    file_path: String,
    state: State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<InstalledPluginInfo, BridgeError> {
    let plugin_manager = get_plugin_manager(&state);
    let host_api = state.get_host_api();
    let path = std::path::PathBuf::from(&file_path);

    plugin_manager
        .install(&path, host_api, app_handle)
        .await
        .map_err(BridgeError::internal)
}

/// Reload a third-party plugin.
/// Emits `plugin-installed` on success.
#[tauri::command]
pub async fn plugin_reload(
    plugin_id: String,
    state: State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<(), BridgeError> {
    let plugin_manager = get_plugin_manager(&state);
    let host_api = state.get_host_api();

    plugin_manager
        .reload(&plugin_id, host_api, app_handle)
        .await
        .map_err(BridgeError::internal)
}

/// Uninstall a third-party plugin.
/// Emits `plugin-uninstalled` on success.
#[tauri::command]
pub async fn plugin_uninstall(
    plugin_id: String,
    state: State<'_, Arc<AppState>>,
    app_handle: tauri::AppHandle,
) -> Result<(), BridgeError> {
    let plugin_manager = get_plugin_manager(&state);
    let host_api = state.get_host_api();

    plugin_manager
        .uninstall(&plugin_id, host_api, app_handle)
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
    let plugin_manager = get_plugin_manager(&state);
    plugin_manager
        .set_enabled(&plugin_id, enabled)
        .map_err(BridgeError::internal)
}

/// Get recent log lines from a plugin's stderr log.
#[tauri::command]
pub async fn plugin_get_logs(
    plugin_id: String,
    tail_lines: Option<usize>,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<String>, BridgeError> {
    let host_manager = get_host_manager(&state)?;

    let log_file = host_manager.log_dir_root.join(format!("{}.log", plugin_id));
    if !log_file.exists() {
        return Ok(Vec::new());
    }

    let content =
        std::fs::read_to_string(&log_file).map_err(|e| BridgeError::internal(e.to_string()))?;
    let lines: Vec<&str> = content.lines().collect();
    let n = tail_lines.unwrap_or(50);
    let start = if lines.len() > n { lines.len() - n } else { 0 };

    Ok(lines[start..].iter().map(|s| s.to_string()).collect())
}
