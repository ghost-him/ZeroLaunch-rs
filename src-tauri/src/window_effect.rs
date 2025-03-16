use tauri::utils::config::WindowEffectsConfig;
use tauri::window::EffectsBuilder;
use tauri::{Manager, WebviewAttributes};
use tauri_utils::TitleBarStyle;
use tokio::runtime;

use crate::modules::config::ui_config::BlurStyle;
use crate::utils::service_locator::ServiceLocator;
use windows::{
    core::*,
    Win32::Graphics::Dwm::{
        DwmExtendFrameIntoClientArea, DwmSetWindowAttribute, DWMWA_BORDER_COLOR,
        DWMWA_WINDOW_CORNER_PREFERENCE, DWMWINDOWATTRIBUTE,
    },
    Win32::UI::Controls::MARGINS,
    Win32::UI::WindowsAndMessaging::*,
};

use windows::Win32::Foundation::HWND;

pub fn enable_window_effect() {
    let state = ServiceLocator::get_state();
    let handle = state.get_main_handle().unwrap();
    let main_window = handle.get_webview_window("main").unwrap();
    let runtime_config = state.get_runtime_config().unwrap();
    let ui_config = runtime_config.get_ui_config();
    let blur_style = ui_config.get_blur_style();
    let mut builder = EffectsBuilder::new();

    if let Some(effect) = match blur_style {
        BlurStyle::None => None,
        BlurStyle::Acrylic => Some(tauri_utils::WindowEffect::Acrylic),
        BlurStyle::Mica => Some(tauri_utils::WindowEffect::Mica),
        BlurStyle::Tabbed => Some(tauri_utils::WindowEffect::Tabbed),
    } {
        builder = builder.effect(effect);
    }

    let effects = builder.build();
    let _ = main_window.set_effects(effects);
}

// 定义圆角类型常量（Windows 11）
const DWMWCP_DEFAULT: u32 = 0;

pub fn update_rounded_corners_and_border(is_dark_mode: bool) -> Result<()> {
    let state = ServiceLocator::get_state();
    let handle = state.get_main_handle().unwrap();
    let main_window = handle.get_webview_window("main").unwrap();
    let hwnd = main_window.hwnd().unwrap();
    unsafe {
        // 设置窗口圆角
        let corner_preference = DWMWCP_DEFAULT; // 使用标准圆角
        DwmSetWindowAttribute(
            hwnd,
            DWMWINDOWATTRIBUTE(DWMWA_WINDOW_CORNER_PREFERENCE.0 as i32),
            &corner_preference as *const _ as *const _,
            std::mem::size_of::<u32>() as u32,
        )?;

        // 设置边框颜色（可以根据需要调整颜色值）

        let border_color: u32 = if is_dark_mode { 0x3d3d3d } else { 0xbdbdbd };
        DwmSetWindowAttribute(
            hwnd,
            DWMWINDOWATTRIBUTE(DWMWA_BORDER_COLOR.0 as i32),
            &border_color as *const _ as *const _,
            std::mem::size_of::<u32>() as u32,
        )?;

        // 创建1px边框
        let margins = MARGINS {
            cxLeftWidth: 1,
            cxRightWidth: 1,
            cyTopHeight: 1,
            cyBottomHeight: 1,
        };

        // 扩展边框到客户区
        DwmExtendFrameIntoClientArea(hwnd, &margins)?;
    }

    Ok(())
}
