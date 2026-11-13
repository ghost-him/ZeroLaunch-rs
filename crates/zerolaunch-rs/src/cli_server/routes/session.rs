use axum::extract::State;
use axum::Json;
use std::sync::Arc;

use crate::state::app_state::AppState;

pub async fn get_mode(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let mode_str = state.get_session_dispatcher().current_view_str();
    Json(serde_json::json!({ "mode": mode_str }))
}

pub async fn get_candidates_count(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let count = state.get_session_dispatcher().get_cached_candidates_count();
    Json(serde_json::json!({ "count": count }))
}
