use tauri::webview::WebviewWindow;

pub fn handle_focus_lost(windows: &[WebviewWindow]) {
    for window in windows {
        if window.is_visible().unwrap_or(false) {
            window
                .hide()
                .unwrap_or_else(|e| eprintln!("无法隐藏窗口：{}", e))
        }
    }
}
