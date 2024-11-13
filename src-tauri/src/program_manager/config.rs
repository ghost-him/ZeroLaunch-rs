use super::super::utils::get_start_menu_paths;
use super::Program;
/// 这个配置信息用于配置该模块与子模块的配置信息
/// 同时，还用于应用启动计数信息的存储
use serde::{Deserialize, Serialize};
use serde_json::from_str;
use std::collections::HashMap;

pub const PINYIN_CONTENT_JS: &str = include_str!("./pinyin.json");

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProgramManagerConfig {
    pub launcher: ProgramLauncherConfig,
    pub loader: ProgramLoaderConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProgramLauncherConfig {}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProgramLoaderConfig {
    /// 保存的要启动的地址
    pub target_paths: Vec<String>,
    /// 禁止的地址
    pub forbidden_paths: Vec<String>,
    /// 禁止的程序关键字
    pub forbidden_program_key: Vec<String>,
    /// 设置程序的固定权重偏移 (key) => (bias, note)
    pub program_bias: HashMap<String, (f64, String)>,
}

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
        let (common, user) = get_start_menu_paths().unwrap();
        ProgramLoaderConfig {
            target_paths: vec![common, user],
            forbidden_paths: Vec::new(),
            forbidden_program_key: Vec::new(),
            program_bias: HashMap::new(),
        }
    }
}
