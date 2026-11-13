//! 前端错误上报 IPC 命令。
//!
//! release 下 WebView2 控制台不可见，前端异常只能经此命令落到后端日志；
//! 仅在 commands 层使用，业务模块不应直接调用。

use crate::commands::bridge_error::BridgeError;
use crate::utils::trace_id::generate_trace_id;
use parking_lot::Mutex;
use serde::Deserialize;
use std::sync::LazyLock;
use std::time::{Duration, Instant};
use tracing::{debug, error, Span};

/// 单条前端错误上报载荷（跨 IPC 反序列化）。
///
/// 由前端 `utils/frontend-logger.ts` 的全局错误处理器构造；
/// 前端已做初步截断，后端仍会再次截断，防止超大字符串淹没日志。
#[derive(Debug, Deserialize)]
pub struct FrontendErrorPayload {
    /// 错误来源（窗口 / 模块），如 `search-window`、`settings-window`。
    #[serde(rename = "scope")]
    pub scope: String,
    /// 错误消息（前端已截断到 2000 字符）。
    #[serde(rename = "message")]
    pub message: String,
    /// 错误堆栈；`None` 表示该错误没有可用堆栈（如 Promise 拒绝的字符串原因）。
    #[serde(rename = "stack", default)]
    pub stack: Option<String>,
    /// 触发错误的页面地址或脚本地址；`None` 表示未知。
    #[serde(rename = "url", default)]
    pub url: Option<String>,
}

/// 前端错误上报的滚动限流窗口。
///
/// 仅在本文件内使用；防止渲染循环或组件重复错误在单位时间内刷爆日志。
struct RateWindow {
    /// 当前窗口的起始时刻。
    start: Instant,
    /// 当前窗口内已放行的上报条数。
    count: u32,
}

/// 前端错误上报限流器（每分钟最多 `MAX_REPORTS_PER_MINUTE` 条）。
static RATE_WINDOW: LazyLock<Mutex<RateWindow>> = LazyLock::new(|| {
    Mutex::new(RateWindow {
        start: Instant::now(),
        count: 0,
    })
});

/// 单个一分钟窗口内允许写入日志的上报条数。
const MAX_REPORTS_PER_MINUTE: u32 = 60;

/// 上报一条前端错误并写入后端日志（error 级，默认 info 级别即可见）。
///
/// 参数：`payload` —— 前端构造的错误载荷。
/// 返回：成功（含被限流丢弃）时 `Ok(())`；本命令没有错误路径。
#[tauri::command]
#[tracing::instrument(skip(payload), fields(trace_id))]
pub async fn report_frontend_error(payload: FrontendErrorPayload) -> Result<(), BridgeError> {
    let trace_id = generate_trace_id();
    Span::current().record("trace_id", trace_id.as_str());

    if !allow_report() {
        debug!(scope = %payload.scope, "前端错误上报已限流，丢弃本条");
        return Ok(());
    }

    let scope = truncate_chars(&payload.scope, 64);
    let message = truncate_chars(&payload.message, 2_000);
    let url = payload.url.as_deref().map(|url| truncate_chars(url, 512));
    let stack = payload
        .stack
        .as_deref()
        .map(|stack| truncate_chars(stack, 8_000));

    error!(
        scope = %scope,
        url = url.as_deref().unwrap_or(""),
        stack = stack.as_deref().unwrap_or(""),
        "前端错误: {message}"
    );
    Ok(())
}

/// 判断本次上报是否在配额内；时间窗口按一分钟滚动重置。
fn allow_report() -> bool {
    let mut window = RATE_WINDOW.lock();
    if window.start.elapsed() >= Duration::from_secs(60) {
        window.start = Instant::now();
        window.count = 0;
    }
    if window.count >= MAX_REPORTS_PER_MINUTE {
        return false;
    }
    window.count += 1;
    true
}

/// 按字符数截断字符串，超出时追加省略号（避免按字节截断破坏 UTF-8）。
fn truncate_chars(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let mut truncated: String = text.chars().take(max_chars).collect();
    truncated.push('…');
    truncated
}
