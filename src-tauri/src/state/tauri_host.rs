//! Tauri 外壳的 `Host` 实现：把内核语义映射到 WebView 窗口、托盘与前端事件。

use crate::core::host::{Host, UpdateMenuState};
use crate::core::i18n::I18nManager;
use crate::sdk::HostApi;
use crate::tray::TrayManager;
use parking_lot::RwLock;
use serde_json::json;
use std::sync::Arc;
use tauri::{App, AppHandle, Emitter, Manager, PhysicalPosition};
use tracing::warn;
use zerolaunch_plugin_api::services::window::MonitorInfo;
use zerolaunch_plugin_api::CandidateId;

/// Tauri 宿主适配器（窗口 + 托盘 + 前端事件）。
pub struct TauriHost {
    app: AppHandle,
    tray: RwLock<Option<Arc<TrayManager>>>,
}

impl TauriHost {
    /// 创建适配器；托盘在 `init_tray` 之后才可用。
    pub fn new(app: AppHandle) -> Self {
        Self {
            app,
            tray: RwLock::new(None),
        }
    }

    /// 创建并初始化托盘，之后通过 Host 端口委托托盘操作。
    pub async fn init_tray(
        &self,
        app: &mut App,
        host_api: Arc<HostApi>,
        i18n: Arc<I18nManager>,
        visible: bool,
    ) {
        let tray = Arc::new(TrayManager::new(host_api, i18n));
        tray.init(app, visible).await;
        *self.tray.write() = Some(tray);
    }

    /// 取托盘管理器副本（调用方不得在持锁期间调用其方法）。
    fn tray(&self) -> Option<Arc<TrayManager>> {
        self.tray.read().clone()
    }
}

impl Host for TauriHost {
    fn hide_main_window(&self) {
        if let Some(window) = self.app.get_webview_window("main") {
            let _ = window.hide();
            let _ = window.emit("handle_focus_lost", ());
        }
    }

    fn show_main_window(&self) {
        if let Some(window) = self.app.get_webview_window("main") {
            let _ = window.show();
            let _ = window.set_focus();
            let _ = window.emit("show_window", ());
        }
    }

    fn is_main_window_visible(&self) -> bool {
        self.app
            .get_webview_window("main")
            .map(|window| window.is_visible().unwrap_or(false))
            .unwrap_or(false)
    }

    fn set_main_window_position(&self, x: i32, y: i32) {
        if let Some(window) = self.app.get_webview_window("main") {
            let _ = window.set_position(PhysicalPosition::new(x, y));
        }
    }

    fn main_window_position(&self) -> Option<(i32, i32)> {
        self.app
            .get_webview_window("main")
            .and_then(|window| window.outer_position().ok())
            .map(|position| (position.x, position.y))
    }

    fn monitor_info(&self) -> Vec<MonitorInfo> {
        self.app
            .get_webview_window("main")
            .and_then(|window| window.available_monitors().ok())
            .map(|monitors| {
                monitors
                    .iter()
                    .map(|monitor| {
                        let position = monitor.position();
                        let size = monitor.size();
                        MonitorInfo {
                            x: position.x,
                            y: position.y,
                            width: size.width,
                            height: size.height,
                            scale_factor: monitor.scale_factor(),
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn show_settings_window(&self) {
        let Some(window) = self.app.get_webview_window("setting_window") else {
            return;
        };
        let _ = window.unminimize();
        if let Err(error) = window.show() {
            warn!("显示设置窗口失败: {error}");
            return;
        }
        // 前台锁限制下 set_focus 可能失败：置顶闪切强制提升 Z 序再取消置顶。
        let _ = window.set_always_on_top(true);
        let _ = window.set_always_on_top(false);
        let _ = window.set_focus();
    }

    fn hide_settings_window(&self) {
        if let Some(window) = self.app.get_webview_window("setting_window") {
            let _ = window.hide();
        }
    }

    fn set_settings_window_title(&self, title: &str) {
        if let Some(window) = self.app.get_webview_window("setting_window") {
            let _ = window.set_title(title);
        }
    }

    fn notify(&self, title: &str, body: &str) {
        crate::utils::notification::notify(&self.app, title, body);
    }

    fn exit(&self) {
        self.app.exit(0);
    }

    fn emit_session_state(&self, event: serde_json::Value) {
        let _ = self.app.emit("session-state", event);
    }

    fn emit_candidates_ready(&self) {
        let _ = self.app.emit("candidates-ready", ());
    }

    fn emit_config_changed(&self, config_component_id: &str, component_type: &str) {
        let _ = self.app.emit(
            "config-changed",
            json!({
                "componentId": config_component_id,
                "componentType": component_type,
            }),
        );
    }

    fn emit_inspector_state_updated(&self) {
        let _ = self.app.emit("inspector-state-updated", ());
    }

    fn emit_icon_updated(&self, candidate_generation: u64, id: CandidateId, icon: &str) {
        let _ = self.app.emit(
            "bridge-icon-updated",
            json!({
                "candidateGeneration": candidate_generation,
                "id": id,
                "icon": icon,
            }),
        );
    }

    fn emit_plugin_installed(&self, plugin_id: &str, name: &str, version: &str) {
        let _ = self.app.emit(
            "plugin-installed",
            json!({
                "pluginId": plugin_id,
                "name": name,
                "version": version,
            }),
        );
    }

    fn emit_plugin_uninstalled(&self, plugin_id: &str) {
        let _ = self.app.emit(
            "plugin-uninstalled",
            json!({
                "pluginId": plugin_id,
            }),
        );
    }

    fn emit_system_theme_changed(&self, theme: &str) {
        let _ = self.app.emit("system-theme-changed", theme);
    }

    fn tray_set_visible(&self, visible: bool) {
        if let Some(tray) = self.tray() {
            tray.set_visible(visible);
        }
    }

    fn tray_update_language(&self) {
        if let Some(tray) = self.tray() {
            tray.update_menu_language();
        }
    }

    fn tray_set_game_mode(&self, checked: bool) {
        if let Some(tray) = self.tray() {
            tray.set_game_mode_checked(checked);
        }
    }

    fn tray_set_update_state(&self, state: UpdateMenuState) {
        if let Some(tray) = self.tray() {
            tray.set_update_state(state);
        }
    }
}
