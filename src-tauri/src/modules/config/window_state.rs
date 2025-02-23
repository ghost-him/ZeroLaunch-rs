use crate::modules::config::{Height, Width};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PartialWindowState {
    pub sys_window_scale_factor: Option<f64>,
    pub sys_window_width: Option<Width>,
    pub sys_window_height: Option<Height>,
}

#[derive(Debug)]
struct WindowStateInner {
    /// 当前屏幕的缩放比例
    sys_window_scale_factor: f64,
    /// 显示器的宽
    sys_window_width: Width,
    /// 显示器的长
    sys_window_height: Height,
}

impl Default for WindowStateInner {
    fn default() -> Self {
        WindowStateInner {
            sys_window_scale_factor: 1.0,
            sys_window_width: 0,
            sys_window_height: 0,
        }
    }
}

impl WindowStateInner {
    pub fn get_sys_window_scale_factor(&self) -> f64 {
        self.sys_window_scale_factor
    }
    /// 显示器的宽
    pub fn get_sys_window_width(&self) -> Width {
        self.sys_window_width
    }
    /// 显示器的长
    pub fn get_sys_window_height(&self) -> Height {
        self.sys_window_height
    }
}
#[derive(Debug)]
pub struct WindowState {
    inner: RwLock<WindowStateInner>,
}

impl Default for WindowState {
    fn default() -> Self {
        WindowState {
            inner: RwLock::new(WindowStateInner::default()),
        }
    }
}

impl WindowState {
    pub fn get_sys_window_scale_factor(&self) -> f64 {
        let inner = self.inner.read();
        inner.get_sys_window_scale_factor()
    }
    /// 显示器的宽
    pub fn get_sys_window_width(&self) -> Width {
        let inner = self.inner.read();
        inner.get_sys_window_width()
    }
    /// 显示器的长
    pub fn get_sys_window_height(&self) -> Height {
        let inner = self.inner.read();
        inner.get_sys_window_height()
    }

    pub fn update(&self, partial_window_state: PartialWindowState) {
        let mut inner = self.inner.write();
        if let Some(sys_window_scale_factor) = partial_window_state.sys_window_scale_factor {
            inner.sys_window_scale_factor = sys_window_scale_factor;
        }

        if let Some(sys_window_height) = partial_window_state.sys_window_height {
            inner.sys_window_height = sys_window_height;
        }

        if let Some(sys_window_width) = partial_window_state.sys_window_width {
            inner.sys_window_width = sys_window_width;
        }
    }
}
