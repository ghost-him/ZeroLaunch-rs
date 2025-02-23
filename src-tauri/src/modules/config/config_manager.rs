use super::ui_config::PartialUiConfig;
use crate::modules::config::app_config::AppConfig;
use crate::modules::config::app_config::PartialAppConfig;
use crate::modules::config::default::CONFIG_DEFAULT;
use crate::modules::config::ui_config::UiConfig;
use crate::modules::config::window_state::PartialWindowState;
use crate::modules::config::window_state::WindowState;
use crate::modules::config::RemoteConfig;
use crate::modules::storage::utils::read_or_create_str;
use crate::program_manager::config::program_manager_config::PartialProgramManagerConfig;
use crate::program_manager::config::program_manager_config::ProgramManagerConfig;
use backtrace::Backtrace;
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
    /// 远程配置文件的存放的地址
    remote_config_path: RwLock<String>,
}

impl RuntimeConfig {
    pub fn new(remote_config_path: String) -> Self {
        let mut result = RuntimeConfig {
            app_config: Arc::new(AppConfig::default()),
            ui_config: Arc::new(UiConfig::default()),
            program_manager_config: Arc::new(ProgramManagerConfig::default()),
            window_state: Arc::new(WindowState::default()),
            remote_config_path: RwLock::new(remote_config_path),
        };
        result
    }

    pub fn load_from_remote_config_path(&self, remote_config_path: Option<String>) {
        if let Some(remote_path) = remote_config_path {
            let mut guard = self.remote_config_path.write();
            *guard = remote_path;
        }
        let partial_config = load_config(&self.remote_config_path.read());
        self.update(partial_config);
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

fn load_config(config_path_str: &str) -> PartialConfig {
    println!("load_config");
    // 读取配置文件
    let config_content = read_or_create_str(&config_path_str, Some(CONFIG_DEFAULT.to_string()))
        .expect("无法读取配置文件");

    let final_config: PartialConfig;
    match serde_json::from_str::<RemoteConfig>(&config_content) {
        Ok(config) => {
            // 如果已经正常的读到文件了，则判断文件是不是正常读取了
            if config.version == RemoteConfig::CURRENT_VERSION {
                final_config = config.config_data;
            } else {
                final_config = RuntimeConfig::new("./".to_string()).to_partial();
            }
        }
        Err(_e) => {
            final_config = RuntimeConfig::new("./".to_string()).to_partial();
        }
    }
    final_config
}
