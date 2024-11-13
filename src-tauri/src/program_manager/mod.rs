pub mod config;
mod pinyin_mapper;
/// 这个模块用于对数据进行储存，加工与处理
///
mod program_launcher;
mod program_loader;
mod search_model;
use config::ProgramManagerConfig;
use lazy_static::lazy_static;
use program_launcher::ProgramLauncher;
use program_loader::ProgramLoader;
use search_model::{SearchModelFn, StandardSearchFn};
use std::sync::Mutex;
use std::{borrow::Borrow, sync::Arc};
/// 应用程序的启动方式
#[derive(Debug)]
enum LaunchMethod {
    /// 通过文件路径来启动
    Path(String),
    /// 通过包族名来启动
    PackageFamilyName(String),
}

impl LaunchMethod {
    /// 这个是用于在文件中存储的全局唯一标识符
    pub fn get_text(&self) -> String {
        match &self {
            LaunchMethod::Path(path) => {
                return path.clone();
            }
            LaunchMethod::PackageFamilyName(name) => {
                return name.clone();
            }
        }
    }

    pub fn is_uwp(&self) -> bool {
        match &self {
            LaunchMethod::Path(_) => {
                return false;
            }
            LaunchMethod::PackageFamilyName(_) => {
                return true;
            }
        }
    }
}

/// 表示一个数据
#[derive(Debug)]
struct Program {
    /// 全局唯一标识符，用于快速索引，用于内存中存储
    pub program_guid: u64,
    /// 展示给用户看的名字
    pub show_name: String,
    /// 这个程序的启动方法
    pub launch_method: LaunchMethod,
    /// 用于计算的字符串
    pub alias: Vec<String>,
    /// 权重固定偏移量
    pub stable_bias: f64,
}

/// 数据处理中心

pub struct ProgramManager {
    /// 当前已经注册的程序
    program_registry: Vec<Arc<Program>>,
    /// 程序加载器
    program_loader: ProgramLoader,
    /// 程序启动器
    program_launcher: ProgramLauncher,
    /// 当前程序的搜索模型（目前写死，后期变成可用户自定义）
    search_fn: SearchModelFn,
}

impl ProgramManager {
    /// 初始化，空
    pub fn new() -> Self {
        ProgramManager {
            program_registry: Vec::new(),
            program_loader: ProgramLoader::new(),
            program_launcher: ProgramLauncher::new(),
            search_fn: StandardSearchFn,
        }
    }
    /// 使用配置信息初始化自身与子模块
    pub fn load_from_config(&mut self, config: &ProgramManagerConfig) {
        let program_loader_config = &config.loader;
        let program_launcher_config = &config.launcher;
        // 初始化子模块
        self.program_loader.load_from_config(&program_loader_config);
        self.program_launcher
            .load_from_config(&program_launcher_config);
        // 从loader中加载程序
        self.program_registry = self.program_loader.load_program();
    }
    /// 使用搜索算法搜索，并给出指定长度的序列
    /// user_input: 用户输入的字符串
    /// result_count: 返回的结果，这个值与 `config.show_item_count` 的值保持一致
    /// 返回值：Vec(应用唯一标识符，展示给用户的名字)
    pub fn update(&self, user_input: &String, result_count: u32) -> Vec<(u64, String)> {
        let mut match_scores: Vec<(f64, u64)> = Vec::new(); // (匹配值，唯一标识符)
        for program in self.program_registry.iter() {
            let score = (self.search_fn)(program.clone(), &user_input);
            match_scores.push((score, program.program_guid));
        }

        match_scores.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        let mut result: Vec<(u64, String)> = Vec::new();

        for &(_, guid) in match_scores.iter().take(result_count as usize) {
            let program = &self.program_registry[guid as usize];
            result.push((program.program_guid, program.show_name.clone()));
        }
        result
    }

    /// 加载搜索模型
    pub fn load_search_fn(&mut self, model: SearchModelFn) {
        self.search_fn = model;
    }
    /// 获取当前程序维护的东西
    pub fn get_program_infos(&self) -> Vec<(String, bool, f64, String)> {
        let mut result = Vec::new();
        for item in &self.program_registry {
            result.push((
                item.show_name.clone(),
                item.launch_method.is_uwp(),
                item.stable_bias,
                item.launch_method.get_text(),
            ));
        }
        result
    }
}

lazy_static! {
    pub static ref PROGRAM_MANAGER: Mutex<ProgramManager> = { Mutex::new(ProgramManager::new()) };
}
