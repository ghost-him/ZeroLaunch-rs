/// 这个配置信息用于配置该模块与子模块的配置信息
/// 同时，还用于应用启动计数信息的存储
use serde::{Deserialize, Serialize};
use serde_json::from_str;

use super::Program;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProgramManagerConfig {
    pub launcher: ProgramLauncherConfig,
    pub loader: ProgramLoaderConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProgramLauncherConfig {}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProgramLoaderConfig {}

impl ProgramManagerConfig {
    pub fn default() -> Self {
        ProgramManagerConfig {
            launcher: ProgramLauncherConfig::default(),
            loader: ProgramLoaderConfig::default(),
        }
    }
}

impl ProgramLauncherConfig {
    pub fn default() -> Self {
        ProgramLauncherConfig {}
    }
}

impl ProgramLoaderConfig {
    pub fn default() -> Self {
        ProgramLoaderConfig {}
    }
}
