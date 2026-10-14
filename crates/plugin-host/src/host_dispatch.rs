//! HostDispatch — handles reverse (plugin→host) RPC calls.
//!
//! Routes host/* methods to the appropriate PluginHandle calls.

use zerolaunch_plugin_protocol::{codes, JsonRpcError};

/// 宿主 → 插件的事件推送通道：把注册的回调经此通道以 JSON-RPC
/// 通知（`plugin/*` 命名空间，无 id）推送给插件进程。
pub trait PluginEventSink: Send + Sync {
    /// 推送一条通知（fire-and-forget，失败仅记录日志）。
    /// 参数：method - `plugin/*` 推送方法名；params - 通知载荷。
    fn notify(&self, method: &str, params: serde_json::Value);
}

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

    /// 注入宿主 → 插件的事件推送通道（重启时重新注入，覆盖旧通道）。
    /// 默认空实现：不推送事件的宿主可忽略。
    fn attach_event_sink(&self, _sink: std::sync::Arc<dyn PluginEventSink>) {}

    /// 通知宿主推送通道已失效，注销该插件的订阅回调并取消其定时器。
    /// 默认空实现。
    fn detach_event_sink(&self) {}
}

impl std::fmt::Debug for dyn HostCallHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HostCallHandler")
    }
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
