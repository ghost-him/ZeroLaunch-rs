use crate::sdk::focus_monitor::types::{FocusCallback, FocusEvent};
use crate::sdk::focus_monitor::FocusMonitor;
use dashmap::DashMap;
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use windows::Win32::Foundation::POINT;
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

/// Windows 平台聚焦监控器实现。
/// 构造即开始监控，不可停止 —— FocusMonitor 是系统强制启用的 push-based 服务。
/// 通过 Tauri 的 `on_window_event` 注册回调，当窗口失去焦点时依次调用所有已注册回调。
pub struct WindowsFocusMonitor {
    callbacks: Arc<DashMap<String, FocusCallback>>,
}

impl WindowsFocusMonitor {
    /// 创建 WindowsFocusMonitor 实例并立即开始监控窗口焦点事件。
    pub fn new(app: Arc<AppHandle>) -> Self {
        let callbacks = Arc::new(DashMap::<String, FocusCallback>::new());
        let callbacks_for_event = callbacks.clone();
        let window = app
            .get_webview_window("main")
            .expect("main window not found");
        let window_for_event = window.clone();

        window.on_window_event(move |event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                for entry in callbacks_for_event.iter() {
                    (entry.value())(FocusEvent::Lost);
                }
            }
            if let tauri::WindowEvent::Focused(focused) = event {
                if !focused {
                    let mut point = POINT { x: 0, y: 0 };
                    unsafe {
                        let _ = GetCursorPos(&mut point);
                    }
                    if let Ok(window_position) = window_for_event.inner_position() {
                        if let Ok(window_size) = window_for_event.inner_size() {
                            let in_window = point.x >= window_position.x
                                && point.x <= window_position.x + window_size.width as i32
                                && point.y >= window_position.y
                                && point.y <= window_position.y + window_size.height as i32;
                            if !in_window {
                                for entry in callbacks_for_event.iter() {
                                    (entry.value())(FocusEvent::Lost);
                                }
                            }
                        }
                    }
                }
            }
        });

        Self { callbacks }
    }
}

impl FocusMonitor for WindowsFocusMonitor {
    fn register_callback(&self, id: &str, callback: FocusCallback) {
        self.callbacks.insert(id.to_string(), callback);
    }

    fn unregister_callback(&self, id: &str) {
        self.callbacks.remove(id);
    }
}
