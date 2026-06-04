//! HostDispatch — handles reverse (plugin→host) RPC calls.
//!
//! Routes host/* methods to the appropriate PluginHandle calls.

use zerolaunch_plugin_protocol::{codes, JsonRpcError};

/// Maps a host/* method name to the corresponding PluginHandle call.
/// The actual PluginHandle type is provided by src-tauri, so this is
/// a trait-based dispatch to avoid coupling plugin-host to src-tauri.
#[async_trait::async_trait]
pub trait HostCallHandler: Send + Sync {
    /// Handle a host/* RPC call. Returns Ok(serde_json::Value) on success.
    async fn handle_host_call(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, JsonRpcError>;
}

/// Helper for creating "method not found" errors.
pub fn method_not_found(method: &str) -> JsonRpcError {
    JsonRpcError::new(
        codes::METHOD_NOT_FOUND,
        format!("host method not found: {}", method),
    )
}

/// Helper for creating "invalid params" errors.
pub fn invalid_params(msg: impl Into<String>) -> JsonRpcError {
    JsonRpcError::new(codes::INVALID_PARAMS, msg)
}
