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
        a.components.iter().all(|c| cm.is_enabled(c.component_id())) && !a.components.is_empty()
    }))
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
