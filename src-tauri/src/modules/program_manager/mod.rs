pub mod config;
pub mod image_loader;
pub mod pinyin_mapper;
pub mod program_launcher;
pub mod program_loader;
pub mod search_model;
pub mod unit;
pub mod window_activator;
use crate::modules::program_manager::image_loader::ImageLoader;
use crate::program_manager::config::program_manager_config::ProgramManagerConfig;
use crate::program_manager::unit::*;
use config::program_manager_config::PartialProgramManagerConfig;
use dashmap::DashMap;
use parking_lot::RwLock;
use program_launcher::ProgramLauncher;
use program_loader::ProgramLoader;
use rayon::prelude::*;
use search_model::remove_repeated_space;
use search_model::{standard_search_fn, SearchModelFn};
use std::sync::Arc;
use std::time::Instant;
use window_activator::WindowActivator;

/// 数据处理中心
#[derive(Debug)]
pub struct ProgramManagerInner {
    /// 当前已经注册的程序
    program_registry: Vec<Arc<Program>>,
    /// 程序加载器
    program_loader: Arc<ProgramLoader>,
    /// 程序启动器
    program_launcher: Arc<ProgramLauncher>,
    /// 当前程序的搜索模型（目前写死，后期变成可用户自定义）
    search_fn: SearchModelFn,
    /// 图标获取器
    image_loader: Arc<ImageLoader>,
    /// 程序查找器(程序的guid, 在registry中的下标)
    program_locater: Arc<DashMap<u64, usize>>,
    /// 窗口唤醒器
    window_activator: Arc<WindowActivator>,
}
#[derive(Debug)]
pub struct ProgramManager {
    inner: RwLock<ProgramManagerInner>,
}

/// 内部搜索结果，包含分数和程序ID
struct SearchMatchResult {
    score: f64,
    program_guid: u64,
}

impl ProgramManager {
    /// 初始化，空
    pub fn new(default_icon_path: String) -> Self {
        ProgramManager {
            inner: RwLock::new(ProgramManagerInner::new(default_icon_path)),
        }
    }
    pub fn get_runtime_data(&self) -> PartialProgramManagerConfig {
        let inner = self.inner.read();
        inner.get_runtime_data()
    }

    /// 使用配置信息初始化自身与子模块
    pub fn load_from_config(&self, config: Arc<ProgramManagerConfig>) {
        let mut inner = self.inner.write();
        inner.load_from_config(config);
    }
    /// 使用搜索算法搜索，并给出指定长度的序列
    /// user_input: 用户输入的字符串
    /// result_count: 返回的结果，这个值与 `config.show_item_count` 的值保持一致
    /// 返回值：Vec(应用唯一标识符，展示给用户的名字)
    pub fn update(&self, user_input: &str, result_count: u32) -> Vec<(u64, String)> {
        let inner = self.inner.read();
        inner.update(user_input, result_count)
    }

    /// 测试算法
    pub fn test_search_algorithm(&self, user_input: &str) -> Vec<SearchTestResult> {
        let inner = self.inner.read();
        inner.test_search_algorithm(user_input)
    }

    /// 加载搜索模型
    pub fn load_search_fn(&self, model: SearchModelFn) {
        let mut inner = self.inner.write();
        inner.load_search_fn(model);
    }
    /// 获取当前程序维护的东西
    pub fn get_program_infos(&self) -> Vec<(String, bool, f64, String, u64)> {
        let mut inner = self.inner.write();
        inner.get_program_infos()
    }
    /// 启动一个程序
    pub fn launch_program(&self, program_guid: u64, is_admin_required: bool) {
        let mut inner = self.inner.write();
        inner.launch_program(program_guid, is_admin_required);
    }
    /// 获取程序的图标，返回使用base64编码的png图片
    pub fn get_icon(&self, program_guid: &u64) -> Vec<u8> {
        let inner = self.inner.read();
        inner.get_icon(program_guid)
    }
    /// 获得当前已保存的程序的个数
    pub fn get_program_count(&self) -> usize {
        let inner = self.inner.read();
        inner.get_program_count()
    }
    /// 测试搜索算法的时间开销
    pub fn test_search_algorithm_time(&self) -> (f64, f64, f64) {
        let inner = self.inner.read();
        inner.test_search_algorithm_time()
    }
    /// 获得加载程序的时间开销
    pub fn get_program_loader_loading_time(&self) -> f64 {
        let inner = self.inner.read();
        inner.get_program_loader_loading_time()
    }
    /// 获得搜索关键字
    pub fn get_search_keywords(&self, show_name: &str) -> Vec<String> {
        let inner = self.inner.read();
        inner.get_search_keywords(show_name)
    }
    /// 唤醒窗口
    pub fn activate_target_program(&self, program_guid: u64) -> bool {
        let inner = self.inner.read();
        inner.activate_target_program(program_guid)
    }
    /// 目标应用程序是不是uwp应用
    pub fn is_uwp_program(&self, program_guid: u64) -> bool {
        let inner = self.inner.read();
        inner.is_uwp_program(program_guid)
    }
}

impl ProgramManagerInner {
    /// 初始化，空
    pub fn new(default_icon_path: String) -> Self {
        ProgramManagerInner {
            program_registry: Vec::new(),
            program_loader: Arc::new(ProgramLoader::new()),
            program_launcher: Arc::new(ProgramLauncher::new()),
            search_fn: standard_search_fn,
            image_loader: Arc::new(ImageLoader::new(default_icon_path)),
            program_locater: Arc::new(DashMap::new()),
            window_activator: Arc::new(WindowActivator::new()),
        }
    }
    pub fn get_runtime_data(&self) -> PartialProgramManagerConfig {
        PartialProgramManagerConfig {
            launcher: Some(self.program_launcher.get_runtime_data()),
            loader: None,
        }
    }

    /// 使用配置信息初始化自身与子模块
    pub fn load_from_config(&mut self, config: Arc<ProgramManagerConfig>) {
        let program_loader_config = &config.get_loader_config();
        let program_launcher_config = &config.get_launcher_config();
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
        // 使用核心搜索算法
        let match_results = self.perform_search(user_input, result_count);
        // 转换为所需的输出格式
        let mut result: Vec<(u64, String)> = Vec::new();
        for match_result in match_results {
            let program = &self.program_registry[match_result.program_guid as usize];
            result.push((program.program_guid, program.show_name.clone()));
        }
        result
    }

    fn perform_search(&self, user_input: &str, result_count: u32) -> Vec<SearchMatchResult> {
        // 预处理用户输入
        let user_input = user_input.to_lowercase();
        let user_input = remove_repeated_space(&user_input);

        let launcher = &self.program_launcher;

        // 计算所有程序的匹配分数
        let mut match_scores: Vec<SearchMatchResult> = self
            .program_registry
            .par_iter()
            .map(|program| {
                // 基础匹配分数
                let mut score = (self.search_fn)(program.clone(), &user_input);
                // 加上固定偏移量
                score += program.stable_bias;
                // 加上动态偏移量
                score += launcher.program_dynamic_value_based_launch_time(program.program_guid);

                SearchMatchResult {
                    score,
                    program_guid: program.program_guid,
                }
            })
            .collect();

        // 按分数降序排序
        match_scores.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // 只保留需要的数量
        match_scores.truncate(result_count as usize);

        match_scores
    }

    /// 测试算法
    pub fn test_search_algorithm(&self, user_input: &str) -> Vec<SearchTestResult> {
        // 使用核心搜索算法
        let match_results = self.perform_search(user_input, self.get_program_count() as u32);

        // 转换为详细的测试结果格式
        let mut results: Vec<SearchTestResult> = Vec::new();
        for match_result in match_results {
            let program = &self.program_registry[match_result.program_guid as usize];
            results.push(SearchTestResult {
                program_name: program.show_name.clone(),
                program_keywords: program.search_keywords.join(", "),
                program_path: program.launch_method.get_text(),
                score: match_result.score,
            });
        }

        results
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

    /// 获得测试当前搜索算法的运行速度(最大值，最小值，平均值)
    pub fn test_search_algorithm_time(&self) -> (f64, f64, f64) {
        let mut max_time: f64 = 0.0;
        let mut min_time: f64 = 5000.0;
        let mut average_time: f64 = 0.0;
        let count = self
            .program_registry
            .iter()
            .flat_map(|program| program.search_keywords.iter())
            .map(|alias| alias.len())
            .max()
            .unwrap_or(0);

        if count == 0 {
            return (0.0, 0.0, 0.0);
        }

        for i in 1..=count {
            let search_text = "a".repeat(i);
            let start = Instant::now();
            self.update(&search_text, 5);
            let duration = start.elapsed();
            let duration_ms = duration.as_secs_f64() * 1000.0;
            max_time = max_time.max(duration_ms);
            min_time = min_time.min(duration_ms);
            average_time += duration_ms;
        }

        average_time /= count as f64;
        (max_time, min_time, average_time)
    }

    /// 获得加载程序的加载时间
    pub fn get_program_loader_loading_time(&self) -> f64 {
        self.program_loader.get_loading_time()
    }

    /// 获得搜索关键字
    pub fn get_search_keywords(&self, show_name: &str) -> Vec<String> {
        self.program_loader.convert_search_keywords(show_name)
    }
    /// 唤醒窗口
    pub fn activate_target_program(&self, program_guid: u64) -> bool {
        let target_program_index = self.program_locater.get(&program_guid).unwrap();
        let target_program = self.program_registry[*(target_program_index.value())].clone();
        self.window_activator
            .activate_target_program(target_program)
    }
    /// 返回目标程序是不是 UWP
    pub fn is_uwp_program(&self, program_guid: u64) -> bool {
        let target_program_index = self.program_locater.get(&program_guid).unwrap();
        let target_program = self.program_registry[*(target_program_index.value())].clone();
        target_program.launch_method.is_uwp()
    }
}
