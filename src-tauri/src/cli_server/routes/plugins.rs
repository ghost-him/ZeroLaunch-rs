use axum::extract::{Path, State};
use axum::Json;
use std::sync::Arc;

use crate::state::app_state::AppState;

/// GET /v1/plugins — list all installed plugins.
pub async fn handle_list(State(state): State<Arc<AppState>>) -> Json<Vec<serde_json::Value>> {
    let infos: Vec<serde_json::Value> = state
        .get_plugin_host_manager()
        .map(|mgr| {
            mgr.adapters
                .iter()
                .map(|entry| {
                    let a = entry.value();
                    serde_json::json!({
                        "pluginId": a.plugin_id,
                        "name": a.manifest.plugin.name,
                        "version": a.manifest.plugin.version,
                        "description": a.manifest.plugin.description,
                        "author": a.manifest.plugin.author,
                        "state": "running",
                        "enabled": true,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Json(infos)
}

/// POST /v1/plugins/install — install from local file.
pub async fn handle_install(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let file_path = payload
        .get("filePath")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if file_path.is_empty() {
        return Json(serde_json::json!({ "error": "filePath is required" }));
    }

    let host_manager = state.get_plugin_host_manager();
    match host_manager {
        Some(mgr) => {
            let plugins_dir = mgr
                .data_dir_root
                .parent()
                .map(|p| p.join("plugins"))
                .unwrap_or_default();
            let path = std::path::PathBuf::from(file_path);
            let result = crate::plugin_loader::installer::install_from_zip(&path, &plugins_dir);
            match result {
                Ok(dir) => Json(serde_json::json!({ "installed": dir.to_string_lossy() })),
                Err(e) => Json(serde_json::json!({ "error": e.to_string() })),
            }
        }
        None => Json(serde_json::json!({ "error": "PluginHostManager not initialized" })),
    }
}

/// POST /v1/plugins/:id/reload
pub async fn handle_reload(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let host_manager = match state.get_plugin_host_manager() {
        Some(m) => m,
        None => return Json(serde_json::json!({ "error": "PluginHostManager not initialized" })),
    };

    // Find the plugin directory
    let plugins_dir = host_manager
        .data_dir_root
        .parent()
        .map(|p| p.join("plugins"))
        .unwrap_or_default();
    let plugin_dir = plugins_dir.join(&id);

    if !plugin_dir.exists() {
        return Json(serde_json::json!({ "error": "Plugin directory not found" }));
    }

    // Unregister and unload
    let session_router = state.get_session_router();
    let config_manager = state.get_config_manager();
    if let Some(adapters) = host_manager.adapters.get(&id) {
        session_router.unregister_plugin(&id);
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
    let _ = host_manager.unload(&id).await;

    // Reload
    let host_api = state.get_host_api();
    let app_handle = state.get_main_handle();
    match crate::plugin_loader::loader::load_plugin(
        &plugin_dir,
        &config_manager,
        session_router,
        &host_manager,
        host_api,
        (*app_handle).clone(),
    )
    .await
    {
        Ok(()) => Json(serde_json::json!({ "status": "reloaded", "pluginId": id })),
        Err(e) => Json(serde_json::json!({ "error": e })),
    }
}

/// POST /v1/plugins/:id/uninstall
pub async fn handle_uninstall(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let host_manager = match state.get_plugin_host_manager() {
        Some(m) => m,
        None => return Json(serde_json::json!({ "error": "PluginHostManager not initialized" })),
    };

    let session_router = state.get_session_router();
    let config_manager = state.get_config_manager();

    if let Some(adapters) = host_manager.adapters.get(&id) {
        session_router.unregister_plugin(&id);
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

    let _ = host_manager.unload(&id).await;
    state.get_host_api().unregister(&id);

    // Remove from filesystem
    let plugins_dir = host_manager
        .data_dir_root
        .parent()
        .map(|p| p.join("plugins"))
        .unwrap_or_default();
    let plugin_dir = plugins_dir.join(&id);
    if plugin_dir.exists() {
        let _ = std::fs::remove_dir_all(&plugin_dir);
    }

    Json(serde_json::json!({ "status": "uninstalled", "pluginId": id }))
}

/// GET /v1/plugins/:id/manifest
pub async fn handle_get_manifest(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let manifest_json = state
        .get_plugin_host_manager()
        .and_then(|mgr| {
            let entry = mgr.adapters.get(&id)?;
            let m = &entry.value().manifest;
            Some(serde_json::json!({
                "pluginId": m.plugin.id,
                "name": m.plugin.name,
                "version": m.plugin.version,
                "description": m.plugin.description,
                "author": m.plugin.author,
                "homepage": m.plugin.homepage,
                "minHostVersion": m.plugin.min_host_version,
                "ui": m.ui.as_ref().map(|ui| serde_json::json!({
                    "panelEntry": ui.panel_entry,
                    "settingsEntry": ui.settings_entry,
                })),
            }))
        })
        .unwrap_or(serde_json::Value::Null);

    Json(manifest_json)
}

/// GET /v1/plugins/:id/logs
pub async fn handle_get_logs(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let logs = state
        .get_plugin_host_manager()
        .map(|mgr| {
            let log_file = mgr.log_dir_root.join(format!("{}.log", id));
            if log_file.exists() {
                std::fs::read_to_string(&log_file).unwrap_or_default()
            } else {
                String::new()
            }
        })
        .unwrap_or_default();

    Json(serde_json::json!({ "logs": logs }))
}
