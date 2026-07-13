/// Plugin Inspector IPC 命令。
/// Inspector 始终初始化，录制开关由 `is_debug_mode` 配置控制。
/// 模拟查询已移至 `debug_simulate_query`（`commands/debug.rs`）。
use crate::bridge_error::BridgeError;
use crate::state::app_state::AppState;
use std::sync::Arc;

/// 返回当前 Inspector 状态：已注册插件列表 + 最近查询日志。
/// 若调试模式未开启，返回 `available: false`。
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub async fn inspector_get_state(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, BridgeError> {
    let trace_id = crate::utils::trace_id::generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());
    let inspector = state.get_inspector().ok_or_else(|| {
        BridgeError::internal("Inspector not initialized").with_trace_id(&trace_id)
    })?;
    if !state.is_debug_mode() {
        return Ok(
            serde_json::json!({"available": false, "message": "调试模式未开启，请在设置中启用"}),
        );
    }
    let config_manager = state.get_config_manager();
    let snapshot = inspector.snapshot(&config_manager);
    Ok(serde_json::to_value(snapshot).unwrap_or_default())
}
