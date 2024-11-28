use super::super::utils::{generate_current_date, get_start_menu_paths};
/// 这个配置信息用于配置该模块与子模块的配置信息
/// 同时，还用于应用启动计数信息的存储
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::VecDeque;
pub const PINYIN_CONTENT_JS: &str = include_str!("./pinyin.json");

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProgramManagerConfig {
    pub launcher: ProgramLauncherConfig,
    pub loader: ProgramLoaderConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProgramLauncherConfig {
    /// 天数,[一个地址的启动次数]
    pub launch_info: VecDeque<HashMap<String, u64>>,
    /// 历史启动次数
    pub history_launch_time: HashMap<String, u64>,
    /// 上次的读取日期
    pub last_update_data: String,
}

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
    /// 是不是要遍历uwp应用
    pub is_scan_uwp_programs: bool,
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
        let mut deque = VecDeque::new();
        deque.push_front(HashMap::new());
        ProgramLauncherConfig {
            launch_info: deque,
            history_launch_time: HashMap::new(),
            last_update_data: generate_current_date(),
        }
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
            is_scan_uwp_programs: true,
        }
    }
}
