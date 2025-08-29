use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
/// 存放与windows相关的工具类函数
use std::path::Path;
use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTOPRIMARY,
};
use windows::Win32::System::Environment::ExpandEnvironmentStringsW;
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetParent, GetWindowRect, WindowFromPoint,
};
/// 将一个字符串转成windows的宽字符
pub fn get_u16_vec<P: AsRef<Path>>(path: P) -> Vec<u16> {
    path.as_ref()
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}
/// 检测当前前台窗口是否处于全屏状态
pub fn is_foreground_fullscreen() -> bool {
    unsafe {
        // 获取当前前台窗口句柄
        let foreground_hwnd = GetForegroundWindow();
        if foreground_hwnd.0 == std::ptr::null_mut() {
            return false;
        }

        // 获取主显示器信息
        let monitor = MonitorFromWindow(foreground_hwnd, MONITOR_DEFAULTTOPRIMARY);
        let mut monitor_info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };

        if !GetMonitorInfoW(monitor, &mut monitor_info).as_bool() {
            return false;
        }

        let screen_rect = monitor_info.rcMonitor;
        let screen_width = screen_rect.right - screen_rect.left;
        let screen_height = screen_rect.bottom - screen_rect.top;

        // 获取左下角坐标所属的顶层窗口
        let left_bottom_point = POINT {
            x: screen_rect.left,
            y: screen_rect.bottom - 1,
        };

        let left_bottom_hwnd = top_window_from_point(left_bottom_point);

        // 如果前台窗口与左下角窗口不同，则不是全屏
        if foreground_hwnd.0 != left_bottom_hwnd.0 {
            return false;
        }

        // 获取前台窗口的尺寸
        let mut window_rect = RECT::default();
        if GetWindowRect(foreground_hwnd, &mut window_rect).is_err() {
            return false;
        }

        // 检查窗口尺寸是否与屏幕尺寸相当
        let window_width = window_rect.right - window_rect.left;
        let window_height = window_rect.bottom - window_rect.top;

        window_width >= screen_width && window_height >= screen_height
    }
}

/// 获取指定坐标点所属的顶层窗口
fn top_window_from_point(point: POINT) -> HWND {
    unsafe {
        let mut hwnd = WindowFromPoint(point);

        // 循环获取父窗口，直到找到顶层窗口
        while let Ok(parent) = GetParent(hwnd) {
            if parent.0 == std::ptr::null_mut() {
                break;
            }
            hwnd = parent;
        }

        hwnd
    }
}

/// 使用 Windows API 展开环境变量
pub fn expand_environment_variables(input: &str) -> Option<String> {
    unsafe {
        // 转换为 UTF-16
        let wide_input: Vec<u16> = OsStr::new(input)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        // 首先获取需要的缓冲区大小
        let required_size = ExpandEnvironmentStringsW(PCWSTR::from_raw(wide_input.as_ptr()), None);

        if required_size == 0 {
            return None;
        }

        // 分配缓冲区并展开
        let mut buffer: Vec<u16> = vec![0; required_size as usize];
        let result =
            ExpandEnvironmentStringsW(PCWSTR::from_raw(wide_input.as_ptr()), Some(&mut buffer));

        if result > 0 && result <= required_size {
            // 移除末尾的 null 终止符
            if let Some(&0) = buffer.last() {
                buffer.pop();
            }
            Some(String::from_utf16_lossy(&buffer))
        } else {
            None
        }
    }
}
