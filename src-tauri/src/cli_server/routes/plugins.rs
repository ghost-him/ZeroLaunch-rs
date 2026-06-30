use axum::extract::{Path, State};
use axum::Json;
use std::sync::Arc;

use crate::state::app_state::AppState;
use zerolaunch_plugin_api::config::Configurable;
use zerolaunch_plugin_host::manager::InstalledPluginInfo;
use zerolaunch_plugin_protocol::Manifest;

/// GET /v1/plugins — 列出所有已安装插件。
pub async fn handle_list(State(state): State<Arc<AppState>>) -> Json<Vec<InstalledPluginInfo>> {
    let pm = state.get_plugin_manager();
    let cm = state.get_config_manager();
    let hm = pm.host_manager();

    Json(hm.list_plugin_info(|a| {
        a.configurables
            .iter()
            .all(|c| cm.is_enabled(c.component_id()))
            && !a.configurables.is_empty()
    }))
}

/// POST /v1/plugins/install — 从本地文件安装。
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

    let pm = state.get_plugin_manager();
    let path = std::path::PathBuf::from(file_path);
    let app_handle = state.get_main_handle();
    match pm.install(&path, app_handle).await {
        Ok(info) => Json(serde_json::json!({
            "installed": info.plugin_id,
            "name": info.name,
            "version": info.version,
        })),
        Err(e) => Json(serde_json::json!({ "error": e })),
    }
}

/// POST /v1/plugins/:id/reload
pub async fn handle_reload(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let pm = state.get_plugin_manager();
    let app_handle = state.get_main_handle();

    match pm.reload(&id, app_handle.clone()).await {
        Ok(()) => Json(serde_json::json!({ "status": "reloaded", "pluginId": id })),
        Err(e) => Json(serde_json::json!({ "error": e })),
    }
}

/// POST /v1/plugins/:id/uninstall
pub async fn handle_uninstall(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let pm = state.get_plugin_manager();
    let app_handle = state.get_main_handle();

    match pm.uninstall(&id, app_handle.clone()).await {
        Ok(()) => Json(serde_json::json!({ "status": "uninstalled", "pluginId": id })),
        Err(e) => Json(serde_json::json!({ "error": e })),
    }
}

/// GET /v1/plugins/:id/manifest
pub async fn handle_get_manifest(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<Option<Manifest>> {
    let pm = state.get_plugin_manager();
    Json(pm.get_manifest(&id))
}

/// GET /v1/plugins/:id/logs
pub async fn handle_get_logs(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let pm = state.get_plugin_manager();
    let logs = pm.get_logs(&id, 50).unwrap_or_default();
    Json(serde_json::json!({ "logs": logs }))
}
