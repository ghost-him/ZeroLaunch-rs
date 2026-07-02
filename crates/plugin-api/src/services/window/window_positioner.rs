use crate::host::error::HostApiError;

/// Platform-independent monitor info.
/// Collected from Tauri AppHandle and passed into the positioner,
/// avoiding a direct Tauri dependency in the SDK trait.
#[derive(Debug, Clone)]
pub struct MonitorInfo {
    /// Monitor top-left X in physical pixels.
    pub x: i32,
    /// Monitor top-left Y in physical pixels.
    pub y: i32,
    /// Monitor width in physical pixels.
    pub width: u32,
    /// Monitor height in physical pixels.
    pub height: u32,
    /// DPI scale factor (1.0 = 96 DPI, 1.5 = 144 DPI, etc.).
    /// Used to convert logical window dimensions to physical pixels.
    pub scale_factor: f64,
}

/// Window position in physical pixel coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowPosition {
    pub x: i32,
    pub y: i32,
}

/// Input parameters for the position calculation.
pub struct PositionRequest {
    /// Whether drag-window mode is enabled (highest priority).
    pub enable_drag_window: bool,
    /// Previously saved window position (only used if enable_drag_window is true).
    pub saved_position: Option<WindowPosition>,
    /// Whether follow-mouse mode is enabled (second priority).
    pub follow_mouse: bool,
    /// Vertical position ratio 0.0–1.0 for default centering.
    pub vertical_position_ratio: f64,
    /// Window width in physical pixels, for horizontal centering.
    pub window_width: i32,
    /// Available monitors collected from the system.
    pub monitors: Vec<MonitorInfo>,
}

/// Calculates the optimal window position from configuration and platform state.
///
/// Three-tier priority (mutually exclusive, first match wins):
/// 1. `enable_drag_window` + valid `saved_position` → use saved position
/// 2. `follow_mouse` → center on the monitor containing the cursor
/// 3. Neither → center on the primary monitor
#[async_trait::async_trait]
pub trait WindowPositioner: Send + Sync {
    /// Compute the optimal window position.
    /// Returns the position in physical pixels, or an error if unrecoverable.
    async fn compute_position(
        &self,
        request: PositionRequest,
    ) -> Result<WindowPosition, HostApiError>;
}
