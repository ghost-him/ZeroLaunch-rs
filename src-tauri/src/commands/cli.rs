//! CLI infrastructure commands — e.g. token access for iframe integration.

use crate::state::app_state::AppState;
use std::sync::Arc;
use tauri::State;

/// Returns the CLI HTTP server connection info (host, port, bearer token)
/// so third-party plugin iframes can call the CLI HTTP API.
#[tauri::command]
pub async fn cli_get_info(state: State<'_, Arc<AppState>>) -> Result<serde_json::Value, String> {
    let token = state.get_cli_token().ok_or("CLI server not started")?;
    Ok(serde_json::json!({
        "host": token.host,
        "port": token.port,
        "token": token.token,
    }))
}
