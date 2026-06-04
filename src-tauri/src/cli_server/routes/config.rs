use axum::extract::{Path, State};
use axum::Json;
use std::sync::Arc;

use crate::state::app_state::AppState;

pub async fn list_components(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let cm = state.get_config_manager();
    let components = cm.get_all_components();
    Json(serde_json::to_value(components).unwrap_or_default())
}

pub async fn get_schema(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let cm = state.get_config_manager();
    let schema = cm.get_component_schema(&id);
    Json(serde_json::to_value(schema).unwrap_or_default())
}

pub async fn get_settings(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let cm = state.get_config_manager();
    let settings = cm.get_settings(&id).unwrap_or(serde_json::Value::Null);
    Json(settings)
}

pub async fn apply_settings(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(settings): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let cm = state.get_config_manager();
    match cm.apply_settings(&id, settings) {
        Ok(()) => Json(serde_json::Value::Null),
        Err(e) => Json(serde_json::json!({ "error": e.to_string() })),
    }
}

pub async fn reset_settings(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let cm = state.get_config_manager();
    match cm.reset_to_default(&id) {
        Ok(()) => Json(serde_json::Value::Null),
        Err(e) => Json(serde_json::json!({ "error": e.to_string() })),
    }
}

pub async fn set_enabled(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let cm = state.get_config_manager();
    let enabled = payload
        .get("isEnabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    match cm.set_enabled(&id, enabled) {
        Ok(()) => Json(serde_json::Value::Null),
        Err(e) => Json(serde_json::json!({ "error": e.to_string() })),
    }
}

pub async fn get_actions(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let cm = state.get_config_manager();
    let actions = cm.get_config_actions(&id);
    Json(serde_json::to_value(actions).unwrap_or_default())
}

pub async fn execute_action(
    State(state): State<Arc<AppState>>,
    Path((id, action)): Path<(String, String)>,
    Json(params): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let cm = state.get_config_manager();
    match cm.execute_config_action(&id, &action, &params) {
        Ok(result) => Json(result),
        Err(e) => Json(serde_json::json!({ "error": e })),
    }
}
