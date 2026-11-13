//! 日志导出 IPC 命令。
//!
//! 保存路径由前端保存对话框选定后传入；本命令把当前日志目录、插件子进程日志与
//! 诊断快照打包为 zip 写入该路径，不读写配置内容、不参与搜索会话。

use crate::commands::bridge_error::{BridgeError, WithTraceId};
use crate::core::config::models::ComponentInfo;
use crate::logging::{
    environment_snapshot, export_log_archive, EnvironmentSnapshot, LogArchiveError,
};
use crate::state::app_state::AppState;
use crate::utils::trace_id::generate_trace_id;
use serde::Serialize;
use serde_json::to_vec_pretty;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::async_runtime::spawn_blocking;
use tauri::State;
use tracing::Span;

/// 诊断快照 —— `diagnostics.json` 的唯一序列化结构。
///
/// 仅限本文件使用：环境事实来自 `EnvironmentSnapshot`，组件状态直接复用
/// `ComponentInfo`（与 `config_get_all_components` 同一权威投影），不复制字段。
#[derive(Debug, Serialize)]
struct DiagnosticsSnapshot {
    /// 环境事实，`flatten` 后与组件字段同级写入 JSON。
    #[serde(flatten)]
    environment: EnvironmentSnapshot,
    /// 当前是否开启调试模式。
    #[serde(rename = "debugMode")]
    debug_mode: bool,
    /// 已注册组件清单（含启用状态与默认启用状态）。
    #[serde(rename = "components")]
    components: Vec<ComponentInfo>,
}

/// 把当前日志目录打包为 zip 写入用户选定的路径。
///
/// 参数：`state` —— 应用状态（读取组件启用状态）；`dest_path` —— 目标 zip 绝对路径
///       （前端保存对话框返回值，父目录须已存在；已存在时覆盖）。
/// 返回：成功时 `Ok(())`；日志系统未初始化、目录为空或读写失败时返回 `BridgeError`。
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub async fn export_logs(
    state: State<'_, Arc<AppState>>,
    dest_path: String,
) -> Result<(), BridgeError> {
    let trace_id = generate_trace_id();
    Span::current().record("trace_id", trace_id.as_str());

    let dest = PathBuf::from(dest_path);
    let diagnostics = build_diagnostics_snapshot(state.inner()).with_trace_id(&trace_id)?;
    // 打包是同步 IO，放到阻塞线程池，避免大日志占用异步运行时。
    let joined = spawn_blocking(move || export_log_archive(&dest, Some(diagnostics))).await;
    // 调度层失败（panic/运行时关闭）与归档失败统一为 LogArchiveError，再进统一错误转换点。
    let archive_result = match joined {
        Ok(result) => result,
        Err(e) => Err(LogArchiveError::Task(e.to_string())),
    };
    archive_result.with_trace_id(&trace_id)
}

/// 组装诊断快照：环境事实 + 组件启用状态（不含配置值与查询词）。
///
/// 参数：`state` —— 应用状态，提供配置管理器与调试模式开关。
/// 返回：成功时返回序列化后的 JSON 字节；序列化失败返回 `BridgeError`。
fn build_diagnostics_snapshot(state: &Arc<AppState>) -> Result<Vec<u8>, BridgeError> {
    let snapshot = DiagnosticsSnapshot {
        environment: environment_snapshot(),
        debug_mode: state.is_debug_mode(),
        components: state.get_config_manager().get_all_components(),
    };
    to_vec_pretty(&snapshot).map_err(|e| BridgeError::internal(format!("序列化诊断快照失败: {e}")))
}

#[cfg(test)]
mod tests {
    use super::DiagnosticsSnapshot;
    use crate::logging::EnvironmentSnapshot;
    use serde_json::{json, to_value, Value};

    /// 构造字段固定的环境快照，仅用于序列化契约测试。
    fn sample_environment() -> EnvironmentSnapshot {
        EnvironmentSnapshot {
            generated_at: "2026-11-14 10:00:00".to_string(),
            app_version: "1.5.5".to_string(),
            git_sha: "abcdef123456".to_string(),
            build_time: "2026-11-14 09:00:00".to_string(),
            rustc: "rustc test".to_string(),
            pid: 42,
            session_id: "deadbeef".to_string(),
            os: "Windows 11".to_string(),
            arch: "x86_64".to_string(),
            features: "custom-protocol".to_string(),
            portable: false,
            webview2: "141.0.0.0".to_string(),
            exe: Some("app.exe".to_string()),
            cwd: Some("C:\\app".to_string()),
            log_dir: Some("C:\\app\\logs".to_string()),
        }
    }

    /// 环境字段必须 flatten 到顶层，组件清单保持在 `components` 键下。
    #[test]
    fn diagnostics_snapshot_keeps_json_shape() {
        let snapshot = DiagnosticsSnapshot {
            environment: sample_environment(),
            debug_mode: false,
            components: Vec::new(),
        };
        let value: Value = to_value(&snapshot).unwrap();

        assert_eq!(value["appVersion"], json!("1.5.5"));
        assert_eq!(value["gitSha"], json!("abcdef123456"));
        assert_eq!(value["webview2"], json!("141.0.0.0"));
        assert_eq!(value["debugMode"], json!(false));
        assert_eq!(value["components"], json!([]));
    }
}
