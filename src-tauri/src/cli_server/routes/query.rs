use axum::extract::State;
use axum::Extension;
use axum::Json;
use serde::Deserialize;
use std::sync::Arc;

use crate::cli_server::middleware::TraceId;
use crate::state::app_state::AppState;
use zerolaunch_plugin_api::QueryChannel;
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
        confirm: false,
    };

    // CLI 为只读辅助路径，响应契约固定为 QueryResponse：流程失败时记录错误并返回空结果
    // （错误详情经日志可观测；UI 通道的错误语义由 bridge_query 的 IPC 错误通道承担）。
    match state
        .get_session_dispatcher()
        .route_query(&trace_id.0, &query, QueryChannel::Cli)
        .await
    {
        Ok(routed) => Json(routed.response),
        Err(e) => {
            tracing::error!(trace_id = %trace_id.0, error = %e, "CLI 查询流程执行失败");
            Json(QueryResponse::Empty)
        }
    }
}
