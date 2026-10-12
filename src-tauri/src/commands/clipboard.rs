//! 剪贴板 IPC 命令。
//!
//! 剪贴板属平台能力，前端不直接调用平台 API，统一经此命令写入；
//! 调用方提供待写入的文本（错误提示、插件日志等已在前端展示的内容）。

use crate::commands::bridge_error::{BridgeError, WithTraceId};
use crate::state::app_state::AppState;
use crate::utils::trace_id::generate_trace_id;
use std::sync::Arc;
use tauri::State;

/// 将文本写入系统剪贴板。
///
/// # Arguments
///
/// * `text` - 待写入剪贴板的文本
///
/// # Returns
///
/// 成功返回 `Ok(())`；剪贴板打开或写入失败返回携带 trace_id 的 `BridgeError`。
#[tauri::command]
#[tracing::instrument(skip(state, text), fields(trace_id))]
pub async fn copy_to_clipboard(
    text: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), BridgeError> {
    let trace_id = generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());
    state
        .get_core_handle()
        .set_clipboard_text(&text)
        .with_trace_id(&trace_id)
}
