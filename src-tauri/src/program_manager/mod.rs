pub mod config;
mod pinyin_mapper;
/// 这个模块用于对数据进行储存，加工与处理
///
mod program_launcher;
mod program_loader;
mod search_model;
use super::image_loader::ImageLoader;
use config::{ProgramLauncherConfig, ProgramManagerConfig};
use dashmap::DashMap;
use lazy_static::lazy_static;
use program_launcher::ProgramLauncher;
use program_loader::ProgramLoader;
use rayon::prelude::*;
use search_model::remove_repeated_space;
use search_model::{standard_search_fn, SearchModelFn};
use std::sync::Arc;
use std::sync::Mutex;
/// 应用程序的启动方式
#[derive(Debug, Clone)]
enum LaunchMethod {
    /// 通过文件路径来启动
    Path(String),
    /// 通过包族名来启动
    PackageFamilyName(String),
    /// 使用默认的启动方式来打开一个文件
    File(String),
}

impl LaunchMethod {
    /// 这个是用于在文件中存储的全局唯一标识符
    pub fn get_text(&self) -> String {
        match &self {
            LaunchMethod::Path(path) => path.clone(),
            LaunchMethod::PackageFamilyName(name) => name.clone(),
            LaunchMethod::File(path) => path.clone(),
        }
    }

    pub fn is_uwp(&self) -> bool {
        match &self {
            LaunchMethod::Path(_) => false,
            LaunchMethod::PackageFamilyName(_) => true,
            LaunchMethod::File(_) => false,
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
    /// 应用程序应该展示的图片的地址
    pub icon_path: String,
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
    /// 图标获取器
    image_loader: ImageLoader,
    /// 程序查找器(程序的guid, 在registry中的下标)
    program_locater: DashMap<u64, usize>,
}

impl Default for ProgramManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgramManager {
    /// 初始化，空
    pub fn new() -> Self {
        ProgramManager {
            program_registry: Vec::new(),
            program_loader: ProgramLoader::new(),
            program_launcher: ProgramLauncher::new(),
            search_fn: standard_search_fn,
            image_loader: ImageLoader::new(),
            program_locater: DashMap::new(),
        }
    }
    /// 将当前的配置写到配置文件中
    pub fn get_launcher_config(&mut self) -> ProgramLauncherConfig {
        self.program_launcher.save_to_config()
    }
    /// 使用配置信息初始化自身与子模块
    pub fn load_from_config(&mut self, config: &ProgramManagerConfig) {
        let program_loader_config = &config.loader;
        let program_launcher_config = &config.launcher;
        // 初始化子模块
        self.program_loader.load_from_config(program_loader_config);
        self.program_launcher
            .load_from_config(program_launcher_config);
        // 从loader中加载程序
        self.program_registry.clear();
        self.program_registry = self.program_loader.load_program();
        // 更新launcher
        self.program_launcher.clear_program_launch_info();
        self.program_locater.clear();
        for (index, program) in self.program_registry.iter().enumerate() {
            self.program_launcher
                .register_program(program.program_guid, program.launch_method.clone());
            self.program_locater.insert(program.program_guid, index);
        }
    }
    /// 使用搜索算法搜索，并给出指定长度的序列
    /// user_input: 用户输入的字符串
    /// result_count: 返回的结果，这个值与 `config.show_item_count` 的值保持一致
    /// 返回值：Vec(应用唯一标识符，展示给用户的名字)
    pub fn update(&self, user_input: &str, result_count: u32) -> Vec<(u64, String)> {
        let user_input = user_input.to_lowercase();
        let user_input = remove_repeated_space(&user_input);
        // (匹配值，唯一标识符)
        let launcher = &self.program_launcher;
        let mut match_scores: Vec<(f64, u64)> = self
            .program_registry
            .par_iter()
            .map(|program| {
                // 当前用户输入与程序的匹配度
                let mut score = (self.search_fn)(program.clone(), &user_input);
                // 程序的固定偏移量
                score += program.stable_bias;
                // 程序的动态偏移量
                score += launcher.program_dynamic_value_based_launch_time(program.program_guid);
                (score, program.program_guid)
            })
            .collect();

        match_scores.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        let mut result: Vec<(u64, String)> = Vec::new();

        for &(_, guid) in match_scores.iter().take(result_count as usize) {
            let program = &self.program_registry[guid as usize];
            result.push((program.program_guid, program.show_name.clone()));
        }
        result
    }

    /// 测试算法
    pub fn test_search_algorithm(&self, user_input: &str) {
        let user_input = user_input.to_lowercase();
        let user_input = remove_repeated_space(&user_input);
        // (匹配值，唯一标识符)
        let launcher = &self.program_launcher;
        let mut match_scores: Vec<(f64, u64)> = self
            .program_registry
            .par_iter()
            .map(|program| {
                // 当前用户输入与程序的匹配度
                let mut score = (self.search_fn)(program.clone(), &user_input);
                // 程序的固定偏移量
                score += program.stable_bias;
                // 程序的动态偏移量
                score += launcher.program_dynamic_value_based_launch_time(program.program_guid);
                (score, program.program_guid)
            })
            .collect();

        match_scores.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        let mut result: Vec<(f64, String)> = Vec::new();

        for (score, guid) in match_scores {
            let program = &self.program_registry[guid as usize];
            result.push((score, program.show_name.clone()));
        }
        println!("{:?}", result);
    }

    /// 加载搜索模型
    pub fn load_search_fn(&mut self, model: SearchModelFn) {
        self.search_fn = model;
    }
    /// 获取当前程序维护的东西
    pub fn get_program_infos(&mut self) -> Vec<(String, bool, f64, String, u64)> {
        let mut result = Vec::new();
        for item in &self.program_registry {
            result.push((
                item.show_name.clone(),
                item.launch_method.is_uwp(),
                item.stable_bias,
                item.launch_method.get_text(),
                self.program_launcher
                    .program_history_launch_time(item.program_guid),
            ));
        }
        result
    }
    /// 启动一个程序
    pub fn launch_program(&mut self, program_guid: u64, is_admin_required: bool) {
        self.program_launcher
            .launch_program(program_guid, is_admin_required);
    }
    /// 获取程序的图标，返回使用base64编码的png图片
    pub fn get_icon(&self, program_guid: &u64) -> Vec<u8> {
        let index = self.program_locater.get(program_guid).unwrap();
        let target_program = &self.program_registry[*(index.value())];

        self.image_loader.load_image(&target_program.icon_path)
    }
    /// 获得当前已保存的程序的个数
    pub fn get_program_count(&self) -> usize {
        self.program_registry.len()
    }
}

lazy_static! {
    pub static ref PROGRAM_MANAGER: Mutex<ProgramManager> = Mutex::new(ProgramManager::new());
}
