use crate::sdk::host_api::HostApiError;
use crate::sdk::window::{MonitorInfo, PositionRequest, WindowPosition, WindowPositioner};
use windows::Win32::Foundation::POINT;
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromPoint, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, GetForegroundWindow};

/// Windows platform window position calculator.
/// Uses Win32 APIs to detect cursor position and match it to available monitors.
pub struct WindowsWindowPositioner;

impl Default for WindowsWindowPositioner {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowsWindowPositioner {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl WindowPositioner for WindowsWindowPositioner {
    async fn compute_position(
        &self,
        request: PositionRequest,
    ) -> Result<WindowPosition, HostApiError> {
        if request.monitors.is_empty() {
            return Err(HostApiError::WindowOperationFailed {
                detail: "没有可用的显示器".to_string(),
            });
        }

        // Strategy 1: drag-window mode — use saved position if still valid
        if request.enable_drag_window {
            if let Some(saved) = request.saved_position {
                if is_position_valid(&saved, &request.monitors) {
                    return Ok(saved);
                }
                // Saved position is stale (e.g. external monitor disconnected);
                // fall through to default centering.
            }
            return Ok(calculate_centered_position(
                &request.monitors[0],
                request.window_width,
                request.vertical_position_ratio,
            ));
        }

        // Strategy 2: follow-mouse — center on whichever monitor contains the cursor
        if request.follow_mouse {
            let best =
                get_best_monitor(&request.monitors).unwrap_or_else(|| request.monitors[0].clone());
            return Ok(calculate_centered_position(
                &best,
                request.window_width,
                request.vertical_position_ratio,
            ));
        }

        // Strategy 3: default — center on primary monitor
        Ok(calculate_centered_position(
            &request.monitors[0],
            request.window_width,
            request.vertical_position_ratio,
        ))
    }
}

/// Determine which monitor best represents where the user is currently working.
///
/// Uses `GetCursorPos` + `MonitorFromPoint` to find the monitor under the cursor.
/// Falls back to `GetForegroundWindow` + `MonitorFromWindow` if the cursor
/// monitor cannot be matched to any entry in the Tauri monitor list.
fn get_best_monitor(monitors: &[MonitorInfo]) -> Option<MonitorInfo> {
    // Primary: cursor position
    let mut cursor_pos = POINT { x: 0, y: 0 };
    if unsafe { GetCursorPos(&mut cursor_pos) }.is_ok() {
        let hmonitor = unsafe { MonitorFromPoint(cursor_pos, MONITOR_DEFAULTTONEAREST) };
        if let Some(m) = find_monitor_by_handle(monitors, hmonitor) {
            return Some(m);
        }
    }

    // Fallback: foreground window
    let hwnd = unsafe { GetForegroundWindow() };
    if !hwnd.0.is_null() {
        let hmonitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
        if let Some(m) = find_monitor_by_handle(monitors, hmonitor) {
            return Some(m);
        }
    }

    None
}

/// Match a Win32 HMONITOR to a Tauri-sourced MonitorInfo by comparing rects.
fn find_monitor_by_handle(
    monitors: &[MonitorInfo],
    hmonitor: windows::Win32::Graphics::Gdi::HMONITOR,
) -> Option<MonitorInfo> {
    let mut mi = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if unsafe { GetMonitorInfoW(hmonitor, &mut mi) }.as_bool() {
        // OK
    } else {
        return None;
    }
    let rc = mi.rcMonitor;
    let w32_x = rc.left;
    let w32_y = rc.top;
    let w32_w = (rc.right - rc.left) as u32;
    let w32_h = (rc.bottom - rc.top) as u32;

    // Exact match first, then fuzzy (within 4px tolerance for DPI rounding)
    monitors
        .iter()
        .find(|m| m.x == w32_x && m.y == w32_y && m.width == w32_w && m.height == w32_h)
        .or_else(|| {
            monitors.iter().find(|m| {
                (m.x - w32_x).abs() <= 4
                    && (m.y - w32_y).abs() <= 4
                    && (m.width as i32 - w32_w as i32).abs() <= 4
                    && (m.height as i32 - w32_h as i32).abs() <= 4
            })
        })
        .cloned()
}

/// Check whether a saved window position still falls within at least one monitor's bounds.
fn is_position_valid(pos: &WindowPosition, monitors: &[MonitorInfo]) -> bool {
    monitors.iter().any(|m| {
        pos.x >= m.x
            && pos.x < m.x + m.width as i32
            && pos.y >= m.y
            && pos.y < m.y + m.height as i32
    })
}

/// Center a window on the given monitor horizontally, positioned vertically by ratio.
///
/// `window_width` is in logical pixels. It is converted to physical pixels
/// via the monitor's `scale_factor` so the result aligns with the physical
/// monitor bounds.
fn calculate_centered_position(
    monitor: &MonitorInfo,
    window_width: i32,
    vertical_ratio: f64,
) -> WindowPosition {
    let phys_width = (window_width as f64 * monitor.scale_factor) as i32;
    let x = monitor.x + (monitor.width as i32 - phys_width) / 2;
    let y = monitor.y + (monitor.height as f64 * vertical_ratio) as i32;
    WindowPosition {
        x: x.max(monitor.x),
        y: y.max(monitor.y),
    }
}
