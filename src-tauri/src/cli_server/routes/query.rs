use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::state::app_state::AppState;

#[derive(Debug, Deserialize)]
pub struct QueryRequest {
    #[serde(rename = "rawQuery", default)]
    pub raw_query: String,
}

#[derive(Debug, Serialize)]
pub struct QueryResponse {
    #[serde(rename = "results")]
    pub results: Vec<serde_json::Value>,
}

pub async fn handle(
    State(state): State<Arc<AppState>>,
    Json(req): Json<QueryRequest>,
) -> Json<serde_json::Value> {
    // Forward to SessionRouter
    let query = zerolaunch_plugin_api::Query {
        id: uuid::Uuid::new_v4().to_string(),
        raw_query: req.raw_query.clone(),
        search_term: req.raw_query.to_lowercase(),
    };

    let response = state
        .get_session_router()
        .route_query(&query.id, &query)
        .await;

    Json(serde_json::to_value(response).unwrap_or_default())
}
