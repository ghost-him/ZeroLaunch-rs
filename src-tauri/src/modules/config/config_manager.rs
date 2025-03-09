use super::ui_config::PartialUiConfig;
use crate::core::storage::utils::read_or_create_str;
use crate::modules::config::app_config::AppConfig;
use crate::modules::config::app_config::PartialAppConfig;
use crate::modules::config::default::CONFIG_DEFAULT;
use crate::modules::config::ui_config::UiConfig;
use crate::modules::config::window_state::PartialWindowState;
use crate::modules::config::window_state::WindowState;
use crate::modules::config::LocalConfig;
use crate::program_manager::config::program_manager_config::PartialProgramManagerConfig;
use crate::program_manager::config::program_manager_config::ProgramManagerConfig;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PartialConfig {
    pub app_config: Option<PartialAppConfig>,
    pub ui_config: Option<PartialUiConfig>,
    pub program_manager_config: Option<PartialProgramManagerConfig>,
    pub window_state: Option<PartialWindowState>,
}

#[derive(Debug)]
pub struct RuntimeConfig {
    app_config: Arc<AppConfig>,
    ui_config: Arc<UiConfig>,
    program_manager_config: Arc<ProgramManagerConfig>,
    window_state: Arc<WindowState>,
}

impl RuntimeConfig {
    pub fn new() -> Self {
        let result = RuntimeConfig {
            app_config: Arc::new(AppConfig::default()),
            ui_config: Arc::new(UiConfig::default()),
            program_manager_config: Arc::new(ProgramManagerConfig::default()),
            window_state: Arc::new(WindowState::default()),
        };
        result
    }

    pub fn update(&self, partial_config: PartialConfig) {
        if let Some(partial_app_config) = partial_config.app_config {
            self.app_config.update(partial_app_config);
        }
        if let Some(partial_ui_config) = partial_config.ui_config {
            self.ui_config.update(partial_ui_config);
        }
        if let Some(partial_program_manager_config) = partial_config.program_manager_config {
            self.program_manager_config
                .update(partial_program_manager_config);
        }
        if let Some(partial_window_state) = partial_config.window_state {
            self.window_state.update(partial_window_state);
        }
    }

    pub fn get_app_config(&self) -> Arc<AppConfig> {
        self.app_config.clone()
    }

    pub fn get_ui_config(&self) -> Arc<UiConfig> {
        self.ui_config.clone()
    }

    pub fn get_program_manager_config(&self) -> Arc<ProgramManagerConfig> {
        self.program_manager_config.clone()
    }

    pub fn get_window_state(&self) -> Arc<WindowState> {
        self.window_state.clone()
    }

    pub fn to_partial(&self) -> PartialConfig {
        PartialConfig {
            app_config: Some(self.app_config.to_partial()),
            ui_config: Some(self.ui_config.to_partial()),
            program_manager_config: Some(self.program_manager_config.to_partial()),
            window_state: None,
        }
    }
}
