use tauri::window::EffectsBuilder;
use tauri::Manager;

use crate::utils::service_locator::ServiceLocator;
use windows::{
    core::*,
    Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWINDOWATTRIBUTE,
    },
};

// 更新窗口的状态
pub fn enable_window_effect() {
    // 1. 更新是不是毛玻璃的效果
    update_blur_effect();
    // 2. 更新圆角的大小
    if let Err(e) = update_rounded_corners() {
        println!("{:?}", e);
    }
}

pub fn update_blur_effect() {
    let state = ServiceLocator::get_state();
    let handle = state.get_main_handle().unwrap();
    let main_window = handle.get_webview_window("main").unwrap();
    let runtime_config = state.get_runtime_config().unwrap();
    let ui_config = runtime_config.get_ui_config();
    let blur_style = ui_config.get_blur_style();
    let mut builder = EffectsBuilder::new();

    if ui_config.get_use_windows_sys_control_radius() {
        if let Some(effect) = match blur_style.as_str() {
            "Acrylic" => Some(tauri_utils::WindowEffect::Acrylic),
            "Mica" => Some(tauri_utils::WindowEffect::Mica),
            "Tabbed" => Some(tauri_utils::WindowEffect::Tabbed),
            _ => None,
        } {
            builder = builder.effect(effect);
        }
    }

    let effects = builder.build();

    if ui_config.get_use_windows_sys_control_radius() {
        let _ = main_window.set_effects(effects);
    } else {
        let _ = main_window.set_effects(None);
    }
}

// 定义圆角类型常量（Windows 11）
const DWMWCP_ROUND: u32 = 2;
const DWMWCP_DONOTROUND: u32 = 1;

/// 更新窗口圆角设置
pub fn update_rounded_corners() -> Result<()> {
    let state = ServiceLocator::get_state();
    let handle = state.get_main_handle().unwrap();
    let main_window = handle.get_webview_window("main").unwrap();
    let hwnd = main_window.hwnd().unwrap();
    let use_windows_sys_control_radius = state
        .get_runtime_config()
        .unwrap()
        .get_ui_config()
        .get_use_windows_sys_control_radius();
    unsafe {
        // 设置窗口圆角
        let corner_preference = if use_windows_sys_control_radius {
            DWMWCP_ROUND
        } else {
            // 不使用圆角
            DWMWCP_DONOTROUND
        };
        DwmSetWindowAttribute(
            hwnd,
            DWMWINDOWATTRIBUTE(DWMWA_WINDOW_CORNER_PREFERENCE.0),
            &corner_preference as *const _ as *const _,
            std::mem::size_of::<u32>() as u32,
        )
        .ok();
    }
    Ok(())
}
