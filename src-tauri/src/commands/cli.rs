//! CLI infrastructure commands — e.g. token access for iframe integration.
use crate::commands::bridge_error::{BridgeError, WithTraceId};
use crate::core::cli_token::CliInfo;
use crate::state::app_state::AppState;
use std::sync::Arc;
use tauri::State;

/// Returns the CLI HTTP server connection info (host, port, bearer token)
/// so third-party plugin iframes can call the CLI HTTP API.
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub async fn cli_get_info(state: State<'_, Arc<AppState>>) -> Result<CliInfo, BridgeError> {
    let trace_id = crate::utils::trace_id::generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());
    let token = state
        .get_cli_token()
        .ok_or_else(|| BridgeError::internal("CLI server not started").with_trace_id(&trace_id))?;
    Ok::<_, BridgeError>(token.to_info()).with_trace_id(&trace_id)
}
