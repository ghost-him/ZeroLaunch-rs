/// Plugin Inspector IPC 命令。
/// 仅在 `inspector` feature 启用时提供完整功能；否则返回 disabled 响应。
use crate::bridge_error::BridgeError;
use crate::state::app_state::AppState;
use std::sync::Arc;

/// 返回当前 Inspector 状态：已注册插件列表 + 最近查询日志。
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub async fn inspector_get_state(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, BridgeError> {
    let trace_id = crate::utils::trace_id::generate_trace_id();
    tracing::Span::current().record("trace_id", tracing::field::display(&trace_id));
    #[cfg(feature = "inspector")]
    {
        let inspector = state.get_inspector().ok_or_else(|| {
            BridgeError::internal("Inspector not initialized").with_trace_id(&trace_id)
        })?;
        let config_manager = state.get_config_manager();
        let snapshot = inspector.snapshot(&config_manager);
        Ok(serde_json::to_value(snapshot).unwrap_or_default())
    }
    #[cfg(not(feature = "inspector"))]
    {
        let _ = state;
        Ok(serde_json::json!({"available": false, "message": "Inspector feature is disabled"}))
    }
}

/// 模拟一次查询，返回原始 QueryResponse（不含图标解析）。
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub async fn inspector_simulate_query(
    state: tauri::State<'_, Arc<AppState>>,
    raw_query: String,
) -> Result<serde_json::Value, BridgeError> {
    let trace_id = format!("sim-{}", crate::utils::trace_id::generate_trace_id());
    tracing::Span::current().record("trace_id", tracing::field::display(&trace_id));
    #[cfg(feature = "inspector")]
    {
        use zerolaunch_plugin_api::plugin::Query;

        let session_router = state.get_session_router();
        let query = Query {
            id: trace_id.clone(),
            raw_query: raw_query.clone(),
            search_term: raw_query.to_lowercase(),
        };
        let response = session_router.route_query(&trace_id, &query).await;
        Ok(serde_json::to_value(&response).unwrap_or_default())
    }
    #[cfg(not(feature = "inspector"))]
    {
        let _ = state;
        let _ = raw_query;
        Err(BridgeError::internal("Inspector feature is disabled").with_trace_id(&trace_id))
    }
}
