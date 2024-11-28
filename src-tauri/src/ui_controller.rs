use std::collections::HashSet;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::OnceLock;
use tauri::webview::WebviewWindow;
use tauri::AppHandle;
use tauri::Emitter;
use tauri::Manager;
use tracing::warn;

pub fn handle_pressed(app_handle: &tauri::AppHandle) {
    let main_window = Arc::new(app_handle.get_webview_window("main").unwrap());
    main_window.show().unwrap();
    main_window.set_focus().unwrap();
    main_window.emit("show_window", ()).unwrap();
}

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
