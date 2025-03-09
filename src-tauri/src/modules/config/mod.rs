use config_manager::PartialConfig;

pub mod app_config;
pub mod config_manager;
pub mod default;
pub mod ui_config;

pub mod window_state;

pub type Width = usize;
pub type Height = usize;
use crate::RuntimeConfig;
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LocalConfig {
    pub version: String,
    pub config_data: PartialConfig,
}

// 当前配置文件的版本
impl LocalConfig {
    pub const CURRENT_VERSION: &str = "2";
}

pub fn save_local_config(partial_config: PartialConfig) -> String {
    let data = LocalConfig {
        version: LocalConfig::CURRENT_VERSION.to_string(),
        config_data: partial_config,
    };

    serde_json::to_string(&data).unwrap()
}

pub fn load_local_config(local_config_data: &str) -> PartialConfig {
    // 读取配置文件
    let final_config: PartialConfig;
    match serde_json::from_str::<LocalConfig>(local_config_data) {
        Ok(config) => {
            // 如果已经正常的读到文件了，则判断文件是不是正常读取了
            if config.version == LocalConfig::CURRENT_VERSION {
                final_config = config.config_data;
            } else {
                final_config = RuntimeConfig::new().to_partial();
            }
        }
        Err(_e) => {
            final_config = RuntimeConfig::new().to_partial();
        }
    }
    final_config
}
