use super::ui_config::PartialUiConfig;
// use crate::modules::config::app_config::AppConfig;
// use crate::modules::config::app_config::PartialAppConfig;
use crate::modules::config::ui_config::UiConfig;
use crate::modules::config::window_state::PartialWindowState;
use crate::modules::config::window_state::WindowState;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PartialRuntimeConfig {
    // pub app_config: Option<PartialAppConfig>,
    pub ui_config: Option<PartialUiConfig>,
    pub window_state: Option<PartialWindowState>,
}

#[derive(Debug)]
pub struct RuntimeConfig {
    // app_config: Arc<AppConfig>,
    ui_config: Arc<UiConfig>,
    window_state: Arc<WindowState>,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeConfig {
    pub fn new() -> Self {
        RuntimeConfig {
            // app_config: Arc::new(AppConfig::default()),
            ui_config: Arc::new(UiConfig::default()),
            window_state: Arc::new(WindowState::default()),
        }
    }

    pub fn update(&self, partial_config: PartialRuntimeConfig) {
        // if let Some(partial_app_config) = partial_config.app_config {
        //     self.app_config.update(partial_app_config);
        // }
        if let Some(partial_ui_config) = partial_config.ui_config {
            self.ui_config.update(partial_ui_config);
        }

        if let Some(partial_window_state) = partial_config.window_state {
            self.window_state.update(partial_window_state);
        }
    }

    // pub fn get_app_config(&self) -> Arc<AppConfig> {
    //     self.app_config.clone()
    // }

    pub fn get_ui_config(&self) -> Arc<UiConfig> {
        self.ui_config.clone()
    }

    pub fn get_window_state(&self) -> Arc<WindowState> {
        self.window_state.clone()
    }

    pub fn to_partial(&self) -> PartialRuntimeConfig {
        PartialRuntimeConfig {
            // app_config: Some(self.app_config.to_partial()),
            ui_config: Some(self.ui_config.to_partial()),
            window_state: None,
        }
    }
}
