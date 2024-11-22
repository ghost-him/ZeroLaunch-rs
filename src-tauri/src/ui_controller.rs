use tauri::webview::WebviewWindow;
use tauri::Emitter;
use tracing::{debug, error, info, trace, warn};
pub fn handle_focus_lost(windows: &[WebviewWindow]) {
    for window in windows {
        if window.is_visible().unwrap_or(false) {
            window
                .hide()
                .unwrap_or_else(|e| warn!("无法隐藏窗口：{}", e));
        }
        if window.label() == "main" {
            window.emit("handle_focus_lost", ()).unwrap();
        }
    }
}
