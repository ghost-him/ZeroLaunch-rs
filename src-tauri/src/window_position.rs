use crate::error::OptionExt;
use crate::error::ResultExt;
use crate::ServiceLocator;
use tauri::Manager;
use tauri::PhysicalPosition;
use tracing::debug;

use windows::Win32::Foundation::POINT;
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromPoint, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, GetForegroundWindow};

/// 获取最佳匹配的显示器
/// 优先使用鼠标所在的显示器，如果失败则尝试使用当前活动窗口所在的显示器
fn get_best_monitor(monitors: &[tauri::Monitor]) -> Option<tauri::Monitor> {
    unsafe {
        // 1. 尝试获取鼠标位置所在的显示器
        let mut point = POINT { x: 0, y: 0 };
        let _ = GetCursorPos(&mut point);
        let hmonitor = MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST);

        if let Some(m) = find_monitor_by_handle(monitors, hmonitor) {
            return Some(m);
        }

        // 2. 如果鼠标位置获取失败（极少情况），尝试获取当前活动窗口所在的显示器
        let hwnd = GetForegroundWindow();
        if !hwnd.0.is_null() {
            let hmonitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
            if let Some(m) = find_monitor_by_handle(monitors, hmonitor) {
                return Some(m);
            }
        }
    }

    None
}

unsafe fn find_monitor_by_handle(
    monitors: &[tauri::Monitor],
    hmonitor: windows::Win32::Graphics::Gdi::HMONITOR,
) -> Option<tauri::Monitor> {
    let mut monitor_info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };

    if GetMonitorInfoW(hmonitor, &mut monitor_info).as_bool() {
        let rect = monitor_info.rcMonitor;
        let win_width = (rect.right - rect.left) as u32;
        let win_height = (rect.bottom - rect.top) as u32;

        for m in monitors {
            let pos = m.position();
            let size = m.size();
            if pos.x == rect.left
                && pos.y == rect.top
                && size.width == win_width
                && size.height == win_height
            {
                return Some(m.clone());
            }
        }

        // 降级策略：如果精确匹配失败，使用模糊匹配（只比较左上角坐标）
        debug!("精确匹配失败，尝试模糊匹配...");
        for m in monitors {
            let pos = m.position();
            if pos.x == rect.left && pos.y == rect.top {
                return Some(m.clone());
            }
        }
    }
    None
}

/// 更新窗口位置（居中到当前显示器）。
/// 窗口大小由前端 useWindowResize 动态控制。
pub fn update_window_position() {
    let state = ServiceLocator::get_state();
    let main_window = state
        .get_main_handle()
        .get_webview_window("main")
        .expect_programming("无法获取主窗口");

    let window_width = 600u32;

    let windows = main_window
        .available_monitors()
        .expect_programming("无法获取可用显示器列表");

    let show_position = if let Some(window) = get_best_monitor(&windows) {
        let window_position = window.position();
        let size = window.size();

        let x = window_position.x + (size.width as i32 - window_width as i32) / 2;
        let y = window_position.y + (size.height as i32) / 3;

        (x, y)
    } else {
        (100, 100)
    };

    main_window
        .set_position(PhysicalPosition::new(show_position.0, show_position.1))
        .expect_programming("无法设置窗口位置");

    debug!("窗口位置已更新");
}
