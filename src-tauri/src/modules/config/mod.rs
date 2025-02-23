use config_manager::PartialConfig;

pub mod app_config;
pub mod config_manager;
pub mod default;
pub mod local_config;
pub mod ui_config;
pub mod window_state;

pub type Width = usize;
pub type Height = usize;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RemoteConfig {
    pub version: String,
    pub config_data: PartialConfig,
}

// 当前配置文件的版本
impl RemoteConfig {
    pub const CURRENT_VERSION: &str = "2";
}

pub fn save_remote_config(partial_config: PartialConfig) -> String {
    let data = RemoteConfig {
        version: RemoteConfig::CURRENT_VERSION.to_string(),
        config_data: partial_config,
    };

    serde_json::to_string(&data).unwrap()
}
