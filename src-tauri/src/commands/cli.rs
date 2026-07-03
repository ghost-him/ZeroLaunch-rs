//! CLI infrastructure commands — e.g. token access for iframe integration.
use crate::bridge_error::BridgeError;
use crate::state::app_state::AppState;
use std::sync::Arc;
use tauri::State;

/// Returns the CLI HTTP server connection info (host, port, bearer token)
/// so third-party plugin iframes can call the CLI HTTP API.
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub async fn cli_get_info(
    state: State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, BridgeError> {
    let trace_id = crate::utils::trace_id::generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());
    let token = state
        .get_cli_token()
        .ok_or_else(|| BridgeError::internal("CLI server not started").with_trace_id(&trace_id))?;
    Ok(serde_json::json!({
        "host": token.host,
        "port": token.port,
        "token": token.token,
    }))
}
