//! 日志导出 IPC 命令。
//!
//! 保存路径由前端保存对话框选定后传入；本命令只负责把当前日志目录打包成 zip
//! 写入该路径，不读写配置、不参与搜索会话。

use crate::commands::bridge_error::{BridgeError, WithTraceId};
use crate::logging::{export_log_archive, LogArchiveError};
use std::path::PathBuf;

/// 把当前日志目录打包为 zip 写入用户选定的路径。
///
/// 参数：`dest_path` —— 目标 zip 绝对路径（前端保存对话框返回值，父目录须已存在）；
///       目标文件已存在时覆盖（覆盖确认由保存对话框负责）。
/// 返回：成功时 `Ok(())`；日志系统未初始化、日志目录为空或读写失败时返回 `BridgeError`。
#[tauri::command]
#[tracing::instrument(fields(trace_id))]
pub async fn export_logs(dest_path: String) -> Result<(), BridgeError> {
    let trace_id = crate::utils::trace_id::generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());

    let dest = PathBuf::from(dest_path);
    // 打包是同步 IO，放到阻塞线程池，避免大日志占用异步运行时。
    let joined = tauri::async_runtime::spawn_blocking(move || export_log_archive(&dest)).await;
    // 调度层失败（panic/运行时关闭）与归档失败统一为 LogArchiveError，再进统一错误转换点。
    let archive_result = match joined {
        Ok(result) => result,
        Err(e) => Err(LogArchiveError::Task(e.to_string())),
    };
    archive_result.with_trace_id(&trace_id)
}
