use std::sync::Arc;
use tauri::webview::WebviewWindow;
use tauri::Emitter;
use tauri::Manager;
use tracing::warn;

use super::service_locator::ServiceLocator;
use super::windows::is_foreground_fullscreen;
use crate::update_window_size_and_position;

pub fn handle_pressed(app_handle: &tauri::AppHandle) {
    // 如果不是全屏情况下才唤醒
    let state = ServiceLocator::get_state();
    let runtime_config = state.get_runtime_config().unwrap();
    let app_config = runtime_config.get_app_config();

    if !app_config.get_is_wake_on_fullscreen() {
        if is_foreground_fullscreen() {
            return;
        }
    }

    update_window_size_and_position();

    let main_window = Arc::new(app_handle.get_webview_window("main").unwrap());
    main_window.show().unwrap();
    main_window.set_focus().unwrap();
    main_window.emit("show_window", ()).unwrap();
    let state = ServiceLocator::get_state();
    state.set_search_bar_visible(true);
}

pub fn handle_focus_lost(main_window: Arc<WebviewWindow>) {
    main_window
        .hide()
        .unwrap_or_else(|e| warn!("无法隐藏窗口：{}", e));
    main_window.emit("handle_focus_lost", ()).unwrap();
    let state = ServiceLocator::get_state();
    state.set_search_bar_visible(false);
}
