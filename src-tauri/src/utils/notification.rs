//! 系统通知发送。
//!
//! 内部功能一律经此发送，不直接调用 `tauri_plugin_notification`：插件在
//! `target/debug`、`target/release` 目录下运行时不会设置 AppUserModelID，
//! 通知会落到 Windows PowerShell 名下（或直接被 Windows 丢弃）。这里始终用应用
//! `identifier` 作为 AUMID，安装版与 dev/便携版归属一致。

use tauri::AppHandle;
use tracing::warn;

/// 向系统发送一条桌面通知（Windows 上为 toast）。
///
/// 参数：`app_handle` —— 取配置里的 `identifier` 作为 Windows AppUserModelID；
/// `title` —— 通知标题；`body` —— 通知正文。
/// 行为：异步提交，不阻塞调用方；发送失败（未安装导致 AUMID 未注册、系统通知被关闭等）
/// 只记录 warn，不影响调用方流程。
pub fn notify(app_handle: &AppHandle, title: &str, body: &str) {
    let mut notification = notify_rust::Notification::new();
    notification
        .summary(title)
        .body(body)
        .app_id(&app_handle.config().identifier);

    crate::runtime::spawn(async move {
        if let Err(error) = notification.show() {
            warn!(error = %error, "发送系统通知失败");
        }
    });
}
