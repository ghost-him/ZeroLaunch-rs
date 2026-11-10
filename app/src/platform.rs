//! GPUI 搜索窗的 Win32 适配。
//!
//! GPUI 只提供窗口内容与激活事件，不暴露显示/隐藏、置顶与跳过任务栏；
//! 这些启动器窗口语义在本模块集中实现，Host 与视图只调用这里的薄封装。

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows::core::BOOL;
use windows::Win32::Foundation::{HWND, LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO,
};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, GetWindowRect, IsIconic, IsWindowVisible, SetForegroundWindow,
    SetWindowLongPtrW, SetWindowPos, ShowWindow, GWL_EXSTYLE, HWND_TOPMOST, SWP_FRAMECHANGED,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_SHOW, SW_SHOWNOACTIVATE, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST,
};
use zerolaunch_plugin_api::services::window::MonitorInfo;

/// 从 GPUI 窗口读取顶层 HWND。
/// 参数：window - 当前 GPUI 窗口。
/// 返回：Windows 平台句柄；非 Windows 或读取失败返回 None。
pub fn window_hwnd(window: &gpui_kit::Window) -> Option<isize> {
    let handle = HasWindowHandle::window_handle(window).ok()?;
    match handle.as_raw() {
        RawWindowHandle::Win32(handle) => Some(handle.hwnd.get()),
        _ => None,
    }
}

/// 把窗口配置为启动器形态：置顶、跳过任务栏、保留无边框样式。
/// 参数：hwnd - 顶层窗口句柄。
/// 返回：无；Win32 调用失败只记录日志。
pub fn configure_launcher(hwnd: isize) {
    if hwnd == 0 {
        return;
    }
    let window = HWND(hwnd as *mut _);
    unsafe {
        let style = GetWindowLongPtrW(window, GWL_EXSTYLE);
        let style = style | WS_EX_TOPMOST.0 as isize | WS_EX_TOOLWINDOW.0 as isize;
        SetWindowLongPtrW(window, GWL_EXSTYLE, style);
        let _ = SetWindowPos(
            window,
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        );
    }
}

/// 显示并前置窗口。
/// 参数：hwnd - 顶层窗口句柄。
/// 返回：无。
pub fn show_window(hwnd: isize) {
    if hwnd == 0 {
        return;
    }
    let window = HWND(hwnd as *mut _);
    unsafe {
        let _ = ShowWindow(window, SW_SHOWNOACTIVATE);
        let _ = SetForegroundWindow(window);
        let _ = ShowWindow(window, SW_SHOW);
    }
}

/// 隐藏窗口。
/// 参数：hwnd - 顶层窗口句柄。
/// 返回：无。
pub fn hide_window(hwnd: isize) {
    if hwnd == 0 {
        return;
    }
    unsafe {
        let _ = ShowWindow(
            HWND(hwnd as *mut _),
            windows::Win32::UI::WindowsAndMessaging::SW_HIDE,
        );
    }
}

/// 窗口当前是否可见且未最小化。
/// 参数：hwnd - 顶层窗口句柄。
/// 返回：可见返回 true。
pub fn is_window_visible(hwnd: isize) -> bool {
    if hwnd == 0 {
        return false;
    }
    let window = HWND(hwnd as *mut _);
    unsafe { IsWindowVisible(window).as_bool() && !IsIconic(window).as_bool() }
}

/// 设置窗口物理坐标，保持尺寸与 Z 序。
/// 参数：hwnd - 顶层窗口句柄；x/y - 物理像素坐标。
/// 返回：无。
pub fn set_window_position(hwnd: isize, x: i32, y: i32) {
    if hwnd == 0 {
        return;
    }
    unsafe {
        let _ = SetWindowPos(
            HWND(hwnd as *mut _),
            Some(HWND_TOPMOST),
            x,
            y,
            0,
            0,
            SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
}

/// 读取窗口左上角物理坐标。
/// 参数：hwnd - 顶层窗口句柄。
/// 返回：窗口存在时返回 (x, y)。
pub fn window_position(hwnd: isize) -> Option<(i32, i32)> {
    if hwnd == 0 {
        return None;
    }
    let mut rect = RECT::default();
    unsafe {
        GetWindowRect(HWND(hwnd as *mut _), &mut rect).ok()?;
    }
    Some((rect.left, rect.top))
}

/// 枚举显示器与有效 DPI。
/// 返回：物理像素边界与缩放因子；枚举失败返回空列表。
pub fn monitor_info() -> Vec<MonitorInfo> {
    let mut monitors = Vec::new();
    unsafe {
        let _ = EnumDisplayMonitors(
            None,
            None,
            Some(collect_monitor),
            LPARAM((&mut monitors as *mut Vec<MonitorInfo>) as isize),
        );
    }
    monitors
}

/// `EnumDisplayMonitors` 回调：把单个显示器写入收集表。
/// 参数：hmonitor - 显示器句柄；其余为 Win32 回调参数；data - 收集表指针。
/// 返回：TRUE 表示继续枚举。
unsafe extern "system" fn collect_monitor(
    hmonitor: HMONITOR,
    _hdc: HDC,
    _rect: *mut RECT,
    data: LPARAM,
) -> BOOL {
    let monitors = &mut *(data.0 as *mut Vec<MonitorInfo>);
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if !GetMonitorInfoW(hmonitor, &mut info).as_bool() {
        return BOOL(1);
    }

    let mut dpi_x = 96u32;
    let mut dpi_y = 96u32;
    let _ = GetDpiForMonitor(hmonitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y);

    monitors.push(MonitorInfo {
        x: info.rcMonitor.left,
        y: info.rcMonitor.top,
        width: (info.rcMonitor.right - info.rcMonitor.left).max(0) as u32,
        height: (info.rcMonitor.bottom - info.rcMonitor.top).max(0) as u32,
        scale_factor: dpi_x as f64 / 96.0,
    });
    BOOL(1)
}
