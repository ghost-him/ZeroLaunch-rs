//! Plugin management IPC commands (third-party plugins).

use crate::plugin_loader::installer;
use crate::state::app_state::AppState;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;
use tracing::{error, info};
use zerolaunch_plugin_api::config::Configurable;
use zerolaunch_plugin_host::manager::{InstalledPluginInfo, RegisteredAdapters};

/// Unregister all adapters for a plugin from the runtime registries.
async fn unregister_plugin_adapters(
    adapters: &RegisteredAdapters,
    session_router: &Arc<crate::plugin_system::SessionRouter>,
    config_manager: &Arc<crate::core::config::ConfigManager>,
) {
    session_router.unregister_plugin(&adapters.plugin_id);
    for ds in &adapters.data_sources {
        session_router
            .unregister_data_source(&ds.component_id)
            .await;
    }
    for ex in &adapters.executors {
        session_router.unregister_executor(&ex.component_id);
    }
    for c in &adapters.configurables {
        config_manager.unregister(&c.component_id);
    }
}

/// List all installed third-party plugins.
#[tauri::command]
pub async fn plugin_list(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<InstalledPluginInfo>, String> {
    let host_manager = state
        .get_plugin_host_manager()
        .ok_or("PluginHostManager not initialized")?;

    let config_manager = state.get_config_manager();
    let infos: Vec<InstalledPluginInfo> = host_manager
        .adapters
        .iter()
        .map(|entry| {
            let adapters = entry.value();
            let process_state = host_manager
                .processes
                .get(&adapters.plugin_id)
                .map(|p| format!("{:?}", *p.state.read()))
                .unwrap_or_else(|| "unknown".to_string());

            // Check if all plugin components are enabled in ConfigManager
            let enabled = adapters
                .configurables
                .first()
                .map(|c| config_manager.is_enabled(c.component_id()))
                .unwrap_or(true);

            InstalledPluginInfo {
                plugin_id: adapters.plugin_id.clone(),
                name: adapters.manifest.plugin.name.clone(),
                version: adapters.manifest.plugin.version.clone(),
                description: adapters.manifest.plugin.description.clone(),
                author: adapters.manifest.plugin.author.clone(),
                state: process_state,
                enabled,
            }
        })
        .collect();

    Ok(infos)
}

/// Get the full manifest of a third-party plugin.
#[tauri::command]
pub async fn plugin_get_manifest(
    plugin_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, String> {
    let host_manager = state
        .get_plugin_host_manager()
        .ok_or("PluginHostManager not initialized")?;

    let adapters = host_manager
        .adapters
        .get(&plugin_id)
        .ok_or(format!("Plugin not found: {}", plugin_id))?;

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
#[tauri::command]
pub async fn plugin_install_local(
    file_path: String,
    state: State<'_, Arc<AppState>>,
) -> Result<InstalledPluginInfo, String> {
    let path = PathBuf::from(&file_path);
    if !path.exists() {
        return Err(format!("File not found: {}", file_path));
    }

    // Determine plugins directory from the host manager
    let host_manager = state
        .get_plugin_host_manager()
        .ok_or("PluginHostManager not initialized")?;
    let plugins_dir = host_manager.plugins_dir();
    std::fs::create_dir_all(&plugins_dir).map_err(|e| e.to_string())?;

    let installed_dir = if path.is_dir() {
        installer::install_from_dir(&path, &plugins_dir).map_err(|e| e.to_string())?
    } else if file_path.ends_with(".zip") {
        installer::install_from_zip(&path, &plugins_dir).map_err(|e| e.to_string())?
    } else {
        return Err("Unsupported file format. Use .zip or directory.".into());
    };

    // Load the newly installed plugin
    let config_manager = state.get_config_manager();
    let session_router = state.get_session_router();
    let host_api = state.get_host_api();

    crate::plugin_loader::loader::load_plugin(
        &installed_dir,
        &config_manager,
        session_router,
        &host_manager,
        host_api,
    )
    .await
    .map_err(|e| format!("Failed to load plugin: {}", e))?;

    // Get info from the newly loaded adapter (avoids re-parsing manifest from disk)
    let plugin_id = installed_dir
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("Invalid plugin directory name")?;
    let adapters = host_manager
        .adapters
        .get(plugin_id)
        .ok_or(format!("Plugin not found after load: {}", plugin_id))?;

    Ok(InstalledPluginInfo {
        plugin_id: adapters.plugin_id.clone(),
        name: adapters.manifest.plugin.name.clone(),
        version: adapters.manifest.plugin.version.clone(),
        description: adapters.manifest.plugin.description.clone(),
        author: adapters.manifest.plugin.author.clone(),
        state: "running".to_string(),
        enabled: true,
    })
}

/// Reload a third-party plugin.
#[tauri::command]
pub async fn plugin_reload(
    plugin_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    info!("Reloading plugin: {}", plugin_id);

    let host_manager = state
        .get_plugin_host_manager()
        .ok_or("PluginHostManager not initialized")?;

    // Get the plugin directory from the manifest
    let adapters = host_manager
        .adapters
        .get(&plugin_id)
        .ok_or(format!("Plugin not found: {}", plugin_id))?;

    let plugins_dir = host_manager.plugins_dir();
    let plugin_dir = plugins_dir.join(&plugin_id);

    // Unregister from registries
    let session_router = state.get_session_router();
    let config_manager = state.get_config_manager();
    unregister_plugin_adapters(&adapters, session_router, &config_manager).await;

    // Unload from host manager
    if let Err(e) = host_manager.unload(&plugin_id).await {
        error!("Unload during reload failed: {}", e);
    }

    // Reload
    let host_api = state.get_host_api();
    crate::plugin_loader::loader::load_plugin(
        &plugin_dir,
        &config_manager,
        session_router,
        &host_manager,
        host_api,
    )
    .await
    .map_err(|e| format!("Reload failed: {}", e))?;

    info!("Plugin {} reloaded successfully", plugin_id);
    Ok(())
}

/// Uninstall a third-party plugin.
#[tauri::command]
pub async fn plugin_uninstall(
    plugin_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    info!("Uninstalling plugin: {}", plugin_id);

    let host_manager = state
        .get_plugin_host_manager()
        .ok_or("PluginHostManager not initialized")?;

    // Unregister from registries first
    let session_router = state.get_session_router();
    let config_manager = state.get_config_manager();

    if let Some(adapters) = host_manager.adapters.get(&plugin_id) {
        unregister_plugin_adapters(&adapters, session_router, &config_manager).await;
    }

    // Unload the process
    if let Err(e) = host_manager.unload(&plugin_id).await {
        error!("Unload during uninstall failed: {}", e);
    }

    // Remove from filesystem
    let plugins_dir = host_manager.plugins_dir();
    let plugin_dir = plugins_dir.join(&plugin_id);
    if plugin_dir.exists() {
        std::fs::remove_dir_all(&plugin_dir)
            .map_err(|e| format!("Cannot remove plugin dir: {}", e))?;
    }

    // Notify host API
    state.get_host_api().unregister(&plugin_id);

    info!("Plugin {} uninstalled successfully", plugin_id);
    Ok(())
}

/// Enable or disable a plugin.
#[tauri::command]
pub async fn plugin_set_enabled(
    plugin_id: String,
    enabled: bool,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    info!("Setting plugin {} enabled={}", plugin_id, enabled);
    // Enable/disable is handled through ConfigManager.
    let config_manager = state.get_config_manager();
    config_manager
        .set_enabled(&plugin_id, enabled)
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Get recent log lines from a plugin's stderr log.
#[tauri::command]
pub async fn plugin_get_logs(
    plugin_id: String,
    tail_lines: Option<usize>,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<String>, String> {
    let host_manager = state
        .get_plugin_host_manager()
        .ok_or("PluginHostManager not initialized")?;

    let log_file = host_manager.log_dir_root.join(format!("{}.log", plugin_id));
    if !log_file.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&log_file).map_err(|e| e.to_string())?;
    let lines: Vec<&str> = content.lines().collect();
    let n = tail_lines.unwrap_or(50);
    let start = if lines.len() > n { lines.len() - n } else { 0 };

    Ok(lines[start..].iter().map(|s| s.to_string()).collect())
}
