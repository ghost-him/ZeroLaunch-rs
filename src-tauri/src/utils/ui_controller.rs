use std::sync::Arc;
use tauri::webview::WebviewWindow;
use tauri::Emitter;
use tauri::Manager;
use tracing::warn;

pub fn handle_pressed(app_handle: &tauri::AppHandle) {
    let main_window = Arc::new(app_handle.get_webview_window("main").unwrap());
    main_window.show().unwrap();
    main_window.set_focus().unwrap();
    main_window.emit("show_window", ()).unwrap();
}

pub fn handle_focus_lost(main_window: Arc<WebviewWindow>) {
    main_window
        .hide()
        .unwrap_or_else(|e| warn!("无法隐藏窗口：{}", e));
    main_window.emit("handle_focus_lost", ()).unwrap();
}
