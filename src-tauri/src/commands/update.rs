//! 更新检查 IPC 命令。
//!
//! 只负责触发检查/读取结果并把状态交给前端；托盘文案、系统通知与发布页跳转
//! 由 `updater::check_and_sync` 统一处理，设置页不重复这些副作用。

use std::sync::Arc;

use tauri::State;

use crate::commands::bridge_error::BridgeError;
use crate::state::app_state::AppState;
use crate::updater::{check_and_sync, UpdateStatus};
use crate::utils::trace_id::generate_trace_id;

/// 主动检查是否有新版本（设置页「检查更新」按钮）。
///
/// 参数：`state` —— 应用状态。返回：本次检查结果；网络失败时结果中的 `error`
/// 有值，不返回 Err（由前端展示失败文案）。不打开浏览器、不发系统通知：
/// 跳转由用户在页面上点击「查看发布页」决定。
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub async fn update_check(state: State<'_, Arc<AppState>>) -> Result<UpdateStatus, BridgeError> {
    let trace_id = generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());

    Ok(check_and_sync(&state, false).await)
}

/// 读取最近一次检查结果（不发起网络请求）。
///
/// 参数：`state` —— 应用状态。返回：共享状态里的检查结论；托盘自动检查的结果
/// 在设置页挂载时可直接展示，避免重复请求 GitHub。
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub fn update_get_status(state: State<'_, Arc<AppState>>) -> Result<UpdateStatus, BridgeError> {
    let trace_id = generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());

    Ok(state.get_update_checker().status())
}
