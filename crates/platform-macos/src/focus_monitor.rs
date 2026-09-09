use dashmap::DashMap;
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use zerolaunch_plugin_api::services::focus_monitor::{FocusCallback, FocusEvent, FocusMonitor};
/// Reports focus loss events from the main Tauri window.
///
/// Used by session management through the platform focus-monitor contract.
pub struct MacosFocusMonitor {
    /// Registered callbacks keyed by their stable service identifier.
    callbacks: Arc<DashMap<String, FocusCallback>>,
}
impl MacosFocusMonitor {
    pub fn new(app: Arc<AppHandle>) -> Self {
        let callbacks = Arc::new(DashMap::<String, FocusCallback>::new());
        let forwarded = callbacks.clone();
        if let Some(window) = app.get_webview_window("main") {
            window.on_window_event(move |event| match event {
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    for callback in forwarded.iter() {
                        (callback.value())(FocusEvent::Lost);
                    }
                }
                tauri::WindowEvent::Focused(false) => {
                    for callback in forwarded.iter() {
                        (callback.value())(FocusEvent::Lost);
                    }
                }
                _ => {}
            });
        } else {
            tracing::warn!("macos focus monitor: \"main\" window not found, focus-loss events will not be delivered");
        }
        Self { callbacks }
    }
}
impl FocusMonitor for MacosFocusMonitor {
    fn register_callback(&self, id: &str, callback: FocusCallback) {
        self.callbacks.insert(id.into(), callback);
    }
    fn unregister_callback(&self, id: &str) {
        self.callbacks.remove(id);
    }
}
