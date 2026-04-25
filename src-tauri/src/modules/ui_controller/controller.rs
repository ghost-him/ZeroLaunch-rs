use tauri::Manager;

use crate::ServiceLocator;

pub type Width = i32;
pub type Height = i32;

pub fn get_window_size() -> (Width, Height) {
    (600, 80)
}

pub fn get_window_render_origin(_vertical_position_ratio: f64) -> (Width, Height) {
    let state = ServiceLocator::get_state();
    let handle = state.get_main_handle();

    if let Some(window) = handle.get_webview_window("main") {
        if let Ok(monitors) = window.available_monitors() {
            if let Some(monitor) = monitors.first() {
                let pos = monitor.position();
                let size = monitor.size();

                let x = pos.x + (size.width as i32 - 600) / 2;
                let y = pos.y + (size.height as i32) / 3;

                return (x, y);
            }
        }
    }

    (100, 100)
}
