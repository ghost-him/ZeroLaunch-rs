use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use super::default::APP_VERSION;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PartialAppConfig {
    pub search_bar_placeholder: Option<String>,
    pub tips: Option<String>,
    pub is_auto_start: Option<bool>,
    pub is_silent_start: Option<bool>,
    pub search_result_count: Option<u32>,
    pub auto_refresh_time: Option<u32>,
    pub launch_new_on_failure: Option<bool>,
    pub is_debug_mode: Option<bool>,
    pub is_esc_hide_window_priority: Option<bool>,
    pub is_enable_drag_window: Option<bool>,
    pub window_position: Option<(i32, i32)>,
}

impl Default for PartialAppConfig {
    fn default() -> Self {
        PartialAppConfig {
            search_bar_placeholder: None,
            tips: None,
            is_auto_start: None,
            is_silent_start: None,
            search_result_count: None,
            auto_refresh_time: None,
            launch_new_on_failure: None,
            is_debug_mode: None,
            is_esc_hide_window_priority: None,
            is_enable_drag_window: None,
            window_position: None,
        }
    }
}

/// 与程序设置有关的，比如是不是要开机自动启动等
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct AppConfigInner {
    /// 自定义搜索栏的提示文本
    #[serde(default = "AppConfigInner::default_search_bar_placeholder")]
    pub search_bar_placeholder: String,
    /// 自定义搜索无结果时的文本
    #[serde(default = "AppConfigInner::default_tips")]
    pub tips: String,
    /// 是不是要开机自启动
    #[serde(default = "AppConfigInner::default_is_auto_start")]
    pub is_auto_start: bool,
    /// 是否静默启动
    #[serde(default = "AppConfigInner::default_is_silent_start")]
    pub is_silent_start: bool,
    /// 搜索结果的数量
    #[serde(default = "AppConfigInner::default_search_result_count")]
    pub search_result_count: u32,
    /// 自动刷新数据库的时间
    #[serde(default = "AppConfigInner::default_auto_refresh_time")]
    pub auto_refresh_time: u32,
    /// 当唤醒失败时启动新实例
    #[serde(default = "AppConfigInner::default_launch_new_on_failure")]
    pub launch_new_on_failure: bool,
    /// 是否是debug模式
    #[serde(default = "AppConfigInner::default_is_debug_mode")]
    pub is_debug_mode: bool,
    /// esc键优先隐藏窗口
    #[serde(default = "AppConfigInner::default_is_esc_hide_window_priority")]
    pub is_esc_hide_window_priority: bool,
    /// 是否要自定义拖动窗口
    #[serde(default = "AppConfigInner::default_is_enable_drag_window")]
    pub is_enable_drag_window: bool,
    /// 上一次的窗口位置
    #[serde(default = "AppConfigInner::default_window_position")]
    pub window_position: (i32, i32),
}

impl Default for AppConfigInner {
    fn default() -> Self {
        Self {
            search_bar_placeholder: Self::default_search_bar_placeholder(),
            tips: Self::default_tips(),
            is_auto_start: Self::default_is_auto_start(),
            is_silent_start: Self::default_is_silent_start(),
            search_result_count: Self::default_search_result_count(),
            auto_refresh_time: Self::default_auto_refresh_time(),
            launch_new_on_failure: Self::default_launch_new_on_failure(),
            is_debug_mode: Self::default_is_debug_mode(),
            is_esc_hide_window_priority: Self::default_is_esc_hide_window_priority(),
            is_enable_drag_window: Self::default_is_enable_drag_window(),
            window_position: Self::default_window_position(),
        }
    }
}

impl AppConfigInner {
    pub(crate) fn default_search_bar_placeholder() -> String {
        "Hello, ZeroLaunch!".to_string()
    }

    pub(crate) fn default_tips() -> String {
        format!("ZeroLaunch-rs v{}", APP_VERSION.clone())
    }

    pub(crate) fn default_is_auto_start() -> bool {
        false
    }

    pub(crate) fn default_is_silent_start() -> bool {
        false
    }

    pub(crate) fn default_search_result_count() -> u32 {
        4
    }

    pub(crate) fn default_auto_refresh_time() -> u32 {
        30
    }

    pub(crate) fn default_launch_new_on_failure() -> bool {
        true
    }

    pub(crate) fn default_is_debug_mode() -> bool {
        false
    }
    pub(crate) fn default_is_esc_hide_window_priority() -> bool {
        false
    }
    pub(crate) fn default_is_enable_drag_window() -> bool {
        false
    }

    pub(crate) fn default_window_position() -> (i32, i32) {
        (0, 0)
    }
}

impl AppConfigInner {
    pub fn update(&mut self, partial_app_config: PartialAppConfig) {
        if let Some(search_bar_placeholder) = partial_app_config.search_bar_placeholder {
            self.search_bar_placeholder = search_bar_placeholder;
        }
        if let Some(tips) = partial_app_config.tips {
            self.tips = tips;
        }
        if let Some(is_auto_start) = partial_app_config.is_auto_start {
            self.is_auto_start = is_auto_start;
        }
        if let Some(is_silent_start) = partial_app_config.is_silent_start {
            self.is_silent_start = is_silent_start;
        }
        if let Some(search_result_count) = partial_app_config.search_result_count {
            self.search_result_count = search_result_count;
        }
        if let Some(launch_new) = partial_app_config.launch_new_on_failure {
            self.launch_new_on_failure = launch_new;
        }
        if let Some(auto_refresh_time) = partial_app_config.auto_refresh_time {
            self.auto_refresh_time = auto_refresh_time;
        }
        if let Some(is_debug_mode) = partial_app_config.is_debug_mode {
            self.is_debug_mode = is_debug_mode;
        }
        if let Some(is_esc_hide) = partial_app_config.is_esc_hide_window_priority {
            self.is_esc_hide_window_priority = is_esc_hide;
        }
        if let Some(enable_drag) = partial_app_config.is_enable_drag_window {
            self.is_enable_drag_window = enable_drag;
        }
        if let Some(position) = partial_app_config.window_position {
            self.window_position = position;
        }
    }
    pub fn to_partial(&self) -> PartialAppConfig {
        PartialAppConfig {
            search_bar_placeholder: Some(self.search_bar_placeholder.clone()),
            tips: Some(self.tips.clone()),
            is_auto_start: Some(self.is_auto_start),
            is_silent_start: Some(self.is_silent_start),
            search_result_count: Some(self.search_result_count),
            launch_new_on_failure: Some(self.launch_new_on_failure),
            auto_refresh_time: Some(self.auto_refresh_time),
            is_debug_mode: Some(self.is_debug_mode),
            is_esc_hide_window_priority: Some(self.is_esc_hide_window_priority),
            is_enable_drag_window: Some(self.is_enable_drag_window),
            window_position: Some(self.window_position),
        }
    }
}
#[derive(Debug)]
pub struct AppConfig {
    inner: RwLock<AppConfigInner>,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            inner: RwLock::new(AppConfigInner::default()),
        }
    }
}

impl AppConfig {
    pub fn update(&self, partial_app_config: PartialAppConfig) {
        let mut inner = self.inner.write();
        inner.update(partial_app_config);
    }

    pub fn get_search_bar_placeholder(&self) -> String {
        let inner = self.inner.read();
        inner.search_bar_placeholder.clone()
    }

    pub fn get_tips(&self) -> String {
        let inner = self.inner.read();
        inner.tips.clone()
    }

    pub fn get_is_auto_start(&self) -> bool {
        let inner = self.inner.read();
        inner.is_auto_start
    }

    pub fn get_is_silent_start(&self) -> bool {
        let inner = self.inner.read();
        inner.is_silent_start
    }

    pub fn get_search_result_count(&self) -> u32 {
        let inner = self.inner.read();
        inner.search_result_count
    }

    pub fn get_auto_refresh_time(&self) -> u32 {
        let inner = self.inner.read();
        inner.auto_refresh_time
    }

    pub fn get_launch_new_on_failure(&self) -> bool {
        let inner = self.inner.read();
        inner.launch_new_on_failure
    }

    pub fn get_is_debug_mode(&self) -> bool {
        let inner = self.inner.read();
        inner.is_debug_mode
    }

    pub fn to_partial(&self) -> PartialAppConfig {
        let inner = self.inner.read();
        inner.to_partial()
    }

    pub fn get_is_esc_hide_window_priority(&self) -> bool {
        let inner = self.inner.read();
        inner.is_esc_hide_window_priority
    }

    pub fn get_is_enable_drag_window(&self) -> bool {
        let inner = self.inner.read();
        inner.is_enable_drag_window
    }

    pub fn get_window_position(&self) -> (i32, i32) {
        let inner = self.inner.read();
        inner.window_position
    }
}
