use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::state::app_state::AppState;

#[derive(Debug, Deserialize)]
pub struct ConfirmRequest {
    #[serde(rename = "candidateId")]
    pub candidate_id: u64,
    #[serde(rename = "actionId")]
    pub action_id: String,
    #[serde(rename = "queryText")]
    pub query_text: String,
    #[serde(rename = "userArgs", default)]
    pub user_args: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase", tag = "status")]
pub enum ConfirmResponse {
    Executed,
    EnterParamPanel {
        candidate_id: u64,
        user_arg_count: usize,
    },
}

pub async fn handle(
    State(state): State<Arc<AppState>>,
    Json(_req): Json<ConfirmRequest>,
) -> Json<serde_json::Value> {
    let router = state.get_session_router();
    // Route confirm through session router
    let trace_id = uuid::Uuid::new_v4().to_string();
    let result = router
        .route_confirm(
            &trace_id,
            &_req.action_id,
            serde_json::json!({
                "candidateId": _req.candidate_id,
                "actionId": _req.action_id,
                "queryText": _req.query_text,
                "userArgs": _req.user_args,
            }),
        )
        .await;

    Json(serde_json::to_value(result).unwrap_or_default())
}
