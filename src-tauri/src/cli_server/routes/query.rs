use axum::extract::State;
use axum::Extension;
use axum::Json;
use serde::Deserialize;
use std::sync::Arc;

use crate::cli_server::middleware::TraceId;
use crate::state::app_state::AppState;
use zerolaunch_plugin_api::QueryResponse;

#[derive(Debug, Deserialize)]
pub struct QueryRequest {
    #[serde(rename = "rawQuery", default)]
    pub raw_query: String,
}

pub async fn handle(
    State(state): State<Arc<AppState>>,
    Extension(trace_id): Extension<TraceId>,
    Json(req): Json<QueryRequest>,
) -> Json<QueryResponse> {
    let query = zerolaunch_plugin_api::Query {
        id: trace_id.0.clone(),
        raw_query: req.raw_query.clone(),
        search_term: req.raw_query.to_lowercase(),
    };

    let response = state
        .get_session_router()
        .route_query(&trace_id.0, &query)
        .await;

    Json(response)
}
