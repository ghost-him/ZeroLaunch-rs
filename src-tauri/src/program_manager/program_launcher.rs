use super::{config::ProgramLauncherConfig, LaunchMethod, Program};
/// 这个类用于启动应用程序，同时还会维护启动次数
///
use std::{alloc::Layout, collections::HashMap};

pub struct ProgramLauncher {
    /// 用于存储目标程序的启动方式
    launch_store: HashMap<u64, LaunchMethod>,
    /// 用户记录当前程序的启动次数
    launch_time: HashMap<u64, u64>,
}

impl ProgramLauncher {
    /// 初始化
    pub fn new() -> ProgramLauncher {
        ProgramLauncher {
            launch_store: HashMap::new(),
            launch_time: HashMap::new(),
        }
    }
    ///使用配置文件初始化
    pub fn load_from_config(&self, config: &ProgramLauncherConfig) {}
    /// 注册一个程序
    pub fn register_program(program_guid: u64, launch_method: LaunchMethod) {}
    /// 通过全局唯一标识符启动程序
    pub fn launch_program(program_guid: u64) {}
    /// 获取当前程序的动态启动次数
    pub fn program_launch_time(program_guid: u64) -> u64 {
        0 as u64
    }
}
