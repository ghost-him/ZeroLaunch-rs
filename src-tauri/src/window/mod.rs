//! 窗口位置工具函数。
//!
//! 从 lib.rs 提取的窗口位置计算、保存、显示器信息收集函数。

use std::sync::Arc;

use serde_json::json;
use tauri::Manager;
use tracing::warn;

use crate::core::config::ConfigManager;
use crate::sdk::HostApi;
use zerolaunch_plugin_api::services::window::{MonitorInfo, PositionRequest, WindowPosition};

/// 准备搜索栏窗口位置：全屏检查 → 读取定位配置 → 计算并设置窗口坐标。
///
/// 返回 `true` 表示定位成功可继续唤醒；
/// 返回 `false` 表示被阻拦（全屏应用且未开启全屏唤醒）。
pub(crate) async fn prepare_window_position(
    config_manager: &Arc<ConfigManager>,
    host_api: &Arc<HostApi>,
    app_handle: &tauri::AppHandle,
) -> bool {
    let wake_on_fullscreen = config_manager
        .get_component_setting("window-behavior-config", "is_wake_on_fullscreen")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !wake_on_fullscreen && crate::utils::windows::is_foreground_fullscreen() {
        return false;
    }

    // 读取窗口定位配置
    let enable_drag = config_manager
        .get_component_setting("window-behavior-config", "is_enable_drag_window")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let follow_mouse = config_manager
        .get_component_setting("window-behavior-config", "show_pos_follow_mouse")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let vertical_ratio = config_manager
        .get_component_setting("appearance-config", "vertical_position_ratio")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.28);
    let window_width = config_manager
        .get_component_setting("appearance-config", "window_width")
        .and_then(|v| v.as_f64())
        .unwrap_or(800.0) as i32;

    // 读取拖拽模式保存的位置
    let saved_position = if enable_drag {
        let x = config_manager
            .get_component_setting("window-behavior-config", "window_position_x")
            .and_then(|v| v.as_f64())
            .map(|v| v as i32)
            .unwrap_or(0);
        let y = config_manager
            .get_component_setting("window-behavior-config", "window_position_y")
            .and_then(|v| v.as_f64())
            .map(|v| v as i32)
            .unwrap_or(0);
        if x != 0 || y != 0 {
            Some(WindowPosition { x, y })
        } else {
            None
        }
    } else {
        None
    };

    // 收集显示器信息并计算窗口位置
    let monitors = collect_monitor_info(app_handle);
    let request = PositionRequest {
        enable_drag_window: enable_drag,
        saved_position,
        follow_mouse,
        vertical_position_ratio: vertical_ratio,
        window_width,
        monitors,
    };

    if let Ok(pos) = host_api.compute_window_position(request).await {
        host_api.set_window_position(pos);
    }

    true
}

/// 若拖拽模式已启用，将当前窗口位置持久化到 ConfigManager。
pub(crate) fn save_window_position_if_drag(
    config_manager: &Arc<ConfigManager>,
    app_handle: &tauri::AppHandle,
) {
    let enable_drag = config_manager
        .get_component_setting("window-behavior-config", "is_enable_drag_window")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !enable_drag {
        return;
    }
    if let Some(window) = app_handle.get_webview_window("main") {
        if let Ok(pos) = window.outer_position() {
            let mut current = config_manager
                .get_settings("window-behavior-config")
                .unwrap_or_else(|| json!({}));
            if let Some(obj) = current.as_object_mut() {
                obj.insert("window_position_x".to_string(), json!(pos.x));
                obj.insert("window_position_y".to_string(), json!(pos.y));
            }
            if let Err(e) = config_manager.apply_settings("window-behavior-config", current) {
                warn!("[save_window_position] 持久化窗口位置失败: {}", e);
            }
        }
    }
}

/// 从 Tauri AppHandle 收集可用显示器信息，供窗口定位使用。
pub(crate) fn collect_monitor_info(app_handle: &tauri::AppHandle) -> Vec<MonitorInfo> {
    app_handle
        .get_webview_window("main")
        .and_then(|w| w.available_monitors().ok())
        .map(|monitors| {
            monitors
                .iter()
                .map(|m| {
                    let pos = m.position();
                    let size = m.size();
                    MonitorInfo {
                        x: pos.x,
                        y: pos.y,
                        width: size.width,
                        height: size.height,
                        scale_factor: m.scale_factor(),
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}
