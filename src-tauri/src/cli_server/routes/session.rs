use axum::extract::State;
use axum::Json;
use std::sync::Arc;

use crate::state::app_state::AppState;

pub async fn get_mode(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let mode = state.get_session_router().current_mode();
    let mode_str = mode.as_str();
    Json(serde_json::json!({ "mode": mode_str }))
}

pub async fn get_candidates_count(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let count = state.get_session_router().get_cached_candidates_count();
    Json(serde_json::json!({ "count": count }))
}
