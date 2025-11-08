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
    let runtime_config = state.get_runtime_config();
    let app_config = runtime_config.get_app_config();

    if !app_config.get_is_wake_on_fullscreen() && is_foreground_fullscreen() {
        return;
    }

    // 在显示搜索栏之前,先保存当前的前台窗口句柄
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
        let hwnd = GetForegroundWindow();
        if !hwnd.0.is_null() {
            state.set_previous_foreground_window(Some(hwnd.0 as isize));
            tracing::debug!("🎯 保存唤醒前的窗口句柄: {}", hwnd.0 as isize);
        } else {
            state.set_previous_foreground_window(None);
            tracing::warn!("⚠️ 无法获取唤醒前的窗口句柄");
        }
    }

    update_window_size_and_position();

    let main_window = match app_handle.get_webview_window("main") {
        Some(window) => Arc::new(window),
        None => {
            warn!("Failed to get main window");
            return;
        }
    };

    if let Err(e) = main_window.show() {
        warn!("Failed to show main window: {}", e);
        return;
    }

    if let Err(e) = main_window.set_focus() {
        warn!("Failed to set focus on main window: {}", e);
        return;
    }

    if let Err(e) = main_window.emit("show_window", ()) {
        warn!("Failed to emit show_window event: {}", e);
    }
    let state = ServiceLocator::get_state();
    state.set_search_bar_visible(true);
}

pub fn handle_focus_lost(main_window: Arc<WebviewWindow>) {
    main_window
        .hide()
        .unwrap_or_else(|e| warn!("无法隐藏窗口：{}", e));
    if let Err(e) = main_window.emit("handle_focus_lost", ()) {
        warn!("Failed to emit handle_focus_lost event: {}", e);
    }
    let state = ServiceLocator::get_state();
    state.set_search_bar_visible(false);
}
