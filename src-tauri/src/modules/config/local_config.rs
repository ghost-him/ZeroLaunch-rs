use crate::modules::storage::windows_utils::get_data_dir_path;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LocalConfig {
    pub remote_config_path: String,
}

pub struct PartialLocalConfig {
    pub remote_config_path: Option<String>,
}

impl LocalConfig {
    pub fn default() -> LocalConfig {
        LocalConfig {
            remote_config_path: get_data_dir_path(),
        }
    }

    pub fn update(&mut self, partial_local_config: PartialLocalConfig) {
        if let Some(remote_config_path) = partial_local_config.remote_config_path {
            self.remote_config_path = remote_config_path;
        }
    }
}
