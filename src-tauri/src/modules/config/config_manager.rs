use super::ui_config::PartialUiConfig;
use crate::modules::config::app_config::AppConfig;
use crate::modules::config::app_config::PartialAppConfig;
use crate::modules::config::ui_config::UiConfig;
use crate::modules::config::window_state::PartialWindowState;
use crate::modules::config::window_state::WindowState;

use crate::modules::everything::config::{EverythingConfig, PartialEverythingConfig};
use crate::modules::icon_manager::config::{IconManagerConfig, PartialIconManagerConfig};
use crate::modules::shortcut_manager::shortcut_config::PartialShortcutConfig;
use crate::modules::shortcut_manager::shortcut_config::ShortcutConfig;
use crate::program_manager::config::program_manager_config::PartialProgramManagerConfig;
use crate::program_manager::config::program_manager_config::ProgramManagerConfig;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PartialRuntimeConfig {
    pub app_config: Option<PartialAppConfig>,
    pub ui_config: Option<PartialUiConfig>,
    pub shortcut_config: Option<PartialShortcutConfig>,
    pub program_manager_config: Option<PartialProgramManagerConfig>,
    pub window_state: Option<PartialWindowState>,
    pub icon_manager_config: Option<PartialIconManagerConfig>,
    pub everything_config: Option<PartialEverythingConfig>,
}

#[derive(Debug)]
pub struct RuntimeConfig {
    app_config: Arc<AppConfig>,
    ui_config: Arc<UiConfig>,
    shortcut_config: Arc<ShortcutConfig>,
    program_manager_config: Arc<ProgramManagerConfig>,
    window_state: Arc<WindowState>,
    icon_manager_config: Arc<IconManagerConfig>,
    everything_config: Arc<EverythingConfig>,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeConfig {
    pub fn new() -> Self {
        RuntimeConfig {
            app_config: Arc::new(AppConfig::default()),
            ui_config: Arc::new(UiConfig::default()),
            shortcut_config: Arc::new(ShortcutConfig::default()),
            program_manager_config: Arc::new(ProgramManagerConfig::default()),
            window_state: Arc::new(WindowState::default()),
            icon_manager_config: Arc::new(IconManagerConfig::default()),
            everything_config: Arc::new(EverythingConfig::default()),
        }
    }

    pub fn update(&self, partial_config: PartialRuntimeConfig) {
        if let Some(partial_app_config) = partial_config.app_config {
            self.app_config.update(partial_app_config);
        }
        if let Some(partial_ui_config) = partial_config.ui_config {
            self.ui_config.update(partial_ui_config);
        }
        if let Some(shortcut_config) = partial_config.shortcut_config {
            self.shortcut_config.update(shortcut_config);
        }
        if let Some(partial_program_manager_config) = partial_config.program_manager_config {
            self.program_manager_config
                .update(partial_program_manager_config);
        }
        if let Some(partial_window_state) = partial_config.window_state {
            self.window_state.update(partial_window_state);
        }
        if let Some(partial_icon_manager_config) = partial_config.icon_manager_config {
            self.icon_manager_config.update(partial_icon_manager_config);
        }
        if let Some(partial_everything_config) = partial_config.everything_config {
            self.everything_config.update(partial_everything_config);
        }
    }

    pub fn get_app_config(&self) -> Arc<AppConfig> {
        self.app_config.clone()
    }

    pub fn get_ui_config(&self) -> Arc<UiConfig> {
        self.ui_config.clone()
    }

    pub fn get_shortcut_config(&self) -> Arc<ShortcutConfig> {
        self.shortcut_config.clone()
    }

    pub fn get_program_manager_config(&self) -> Arc<ProgramManagerConfig> {
        self.program_manager_config.clone()
    }

    pub fn get_window_state(&self) -> Arc<WindowState> {
        self.window_state.clone()
    }

    pub fn get_icon_manager_config(&self) -> Arc<IconManagerConfig> {
        self.icon_manager_config.clone()
    }

    pub fn get_everything_config(&self) -> Arc<EverythingConfig> {
        self.everything_config.clone()
    }

    pub fn to_partial(&self) -> PartialRuntimeConfig {
        PartialRuntimeConfig {
            app_config: Some(self.app_config.to_partial()),
            ui_config: Some(self.ui_config.to_partial()),
            shortcut_config: Some(self.shortcut_config.to_partial()),
            program_manager_config: Some(self.program_manager_config.to_partial()),
            window_state: None,
            icon_manager_config: Some(self.icon_manager_config.to_partial()),
            everything_config: Some(self.everything_config.to_partial()),
        }
    }
}
