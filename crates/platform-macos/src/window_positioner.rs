use async_trait::async_trait;
use zerolaunch_plugin_api::host::HostApiError;
use zerolaunch_plugin_api::services::window::{
    MonitorInfo, PositionRequest, WindowPosition, WindowPositioner,
};
/// Computes search-window positions across macOS monitors.
///
/// Used by the Tauri window setup and repositioning logic.
pub struct MacosWindowPositioner;
impl MacosWindowPositioner {
    pub fn new() -> Self {
        Self
    }
}
impl Default for MacosWindowPositioner {
    fn default() -> Self {
        Self::new()
    }
}
fn valid(position: WindowPosition, monitors: &[MonitorInfo]) -> bool {
    monitors.iter().any(|m| {
        position.x >= m.x
            && position.x < m.x + m.width as i32
            && position.y >= m.y
            && position.y < m.y + m.height as i32
    })
}
fn center(monitor: &MonitorInfo, width: i32, ratio: f64) -> WindowPosition {
    WindowPosition {
        x: monitor.x + (monitor.width as i32 - width) / 2,
        y: monitor.y + (monitor.height as f64 * ratio.clamp(0.0, 1.0)) as i32,
    }
}
#[async_trait]
impl WindowPositioner for MacosWindowPositioner {
    async fn compute_position(
        &self,
        request: PositionRequest,
    ) -> Result<WindowPosition, HostApiError> {
        let monitor =
            request
                .monitors
                .first()
                .ok_or_else(|| HostApiError::WindowOperationFailed {
                    detail: "no monitors are available".into(),
                })?;
        if request.enable_drag_window {
            if let Some(saved) = request
                .saved_position
                .filter(|p| valid(*p, &request.monitors))
            {
                return Ok(saved);
            }
        }
        Ok(center(
            monitor,
            request.window_width,
            request.vertical_position_ratio,
        ))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn monitor() -> MonitorInfo {
        MonitorInfo {
            x: 0,
            y: 0,
            width: 1440,
            height: 900,
            scale_factor: 2.0,
        }
    }
    #[test]
    fn centers_within_primary_monitor() {
        assert_eq!(
            center(&monitor(), 800, 0.28),
            WindowPosition { x: 320, y: 252 }
        );
    }
    #[test]
    fn recognizes_negative_coordinate_monitor() {
        assert!(valid(
            WindowPosition { x: -100, y: 20 },
            &[MonitorInfo {
                x: -1440,
                y: 0,
                width: 1440,
                height: 900,
                scale_factor: 2.0
            }]
        ));
    }
}
