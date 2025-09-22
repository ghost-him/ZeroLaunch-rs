pub mod config;
pub mod image_loader;
pub mod localization_translation;
pub mod pinyin_mapper;
pub mod program_launcher;
pub mod program_loader;
pub mod search_model;
pub mod semantic_manager;
use crate::program_manager::search_engine::TraditionalSearchEngine;
pub mod search_engine;
pub mod unit;
pub mod window_activator;
use crate::core::image_processor::ImageProcessor;
use crate::error::OptionExt;
use crate::modules::program_manager::config::program_manager_config::RuntimeProgramConfig;
use crate::modules::program_manager::search_engine::{SearchEngine, SemanticSearchEngine};
use crate::program_manager::config::program_manager_config::ProgramManagerConfig;
use crate::program_manager::search_model::*;
use crate::program_manager::semantic_manager::SemanticManager;
use crate::program_manager::unit::*;
use config::program_manager_config::PartialProgramManagerConfig;
use dashmap::DashMap;
use image_loader::ImageLoader;
use program_launcher::ProgramLauncher;
use program_loader::ProgramLoader;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use window_activator::WindowActivator;

/// 程序管理器 - 使用细粒度锁优化并发性能
#[derive(Debug)]
pub struct ProgramManager {
    /// 当前已经注册的程序
    program_registry: Arc<RwLock<Vec<Arc<Program>>>>,
    /// 程序查找器(程序的guid, 在registry中的下标)
    program_locater: Arc<DashMap<u64, usize>>,
    /// 程序加载器
    program_loader: Arc<ProgramLoader>,
    /// 程序启动器
    program_launcher: Arc<ProgramLauncher>,
    /// 当前程序的搜索引擎
    search_engine: Arc<RwLock<Arc<dyn SearchEngine>>>,
    /// 图标获取器
    image_loader: Arc<ImageLoader>,
    /// 窗口唤醒器
    window_activator: Arc<WindowActivator>,
    /// 语义生成器
    semantic_manager: Option<Arc<SemanticManager>>,
}

/// 内部搜索结果，包含分数和程序ID
#[derive(Debug)]
pub(crate) struct SearchMatchResult {
    score: f64,
    program_guid: u64,
}

impl ProgramManager {
    /// 初始化，空
    pub fn new(runtime_program_config: RuntimeProgramConfig) -> Self {
        let semantic_manager = Arc::new(SemanticManager::new(runtime_program_config.model_manager));
        ProgramManager {
            program_registry: Arc::new(RwLock::new(Vec::new())),
            program_loader: Arc::new(ProgramLoader::new(Some(semantic_manager.clone()))),
            program_launcher: Arc::new(ProgramLauncher::new()),
            search_engine: Arc::new(RwLock::new(Arc::new(TraditionalSearchEngine::default()))),
            image_loader: Arc::new(ImageLoader::new(runtime_program_config.image_loader_config)),
            program_locater: Arc::new(DashMap::new()),
            window_activator: Arc::new(WindowActivator::new()),
            semantic_manager: Some(semantic_manager),
        }
    }
    pub async fn get_runtime_data(&self) -> PartialProgramManagerConfig {
        PartialProgramManagerConfig {
            launcher: Some(self.program_launcher.get_runtime_data()),
            loader: None,
            image_loader: None,
            search_model: None,
        }
    }

    /// 使用配置信息初始化自身与子模块
    pub async fn load_from_config(&self, config: Arc<ProgramManagerConfig>) {
        let program_loader_config = &config.get_loader_config();
        let program_launcher_config = &config.get_launcher_config();
        let image_loader_config = &config.get_image_loader_config();
        // 初始化子模块
        self.image_loader
            .load_from_config(image_loader_config)
            .await;
        self.program_loader.load_from_config(program_loader_config);
        // 加载程序数据
        let new_programs = self.program_loader.load_program();

        // 清空并更新程序注册表
        let mut program_registry = self.program_registry.write().await;
        program_registry.clear();
        *program_registry = new_programs;

        let programs_to_register: Vec<(u64, LaunchMethod)> = program_registry
            .iter()
            .map(|program| (program.program_guid, program.launch_method.clone()))
            .collect();

        // 原子性地加载配置和注册程序
        self.program_launcher
            .load_and_register_programs(program_launcher_config, &programs_to_register);

        // 更新定位器
        self.program_locater.clear();
        for (index, program) in program_registry.iter().enumerate() {
            self.program_locater.insert(program.program_guid, index);
        }

        // 更新搜索模型
        let search_config = config.get_search_model_config(); // 返回 SearchModelConfig

        let search_engine: Arc<dyn SearchEngine> = if search_config.is_traditional_search() {
            let new_search_model = SearchModelFactory::create_scorer(search_config, None);
            Arc::new(TraditionalSearchEngine::new(Arc::new(new_search_model)))
        } else {
            Arc::new(SemanticSearchEngine::new(
                self.semantic_manager
                    .clone()
                    .expect_programming("语义模型未初始化"),
            ))
        };

        let mut search_engine_lock = self.search_engine.write().await;
        *search_engine_lock = search_engine;
    }

    /// 使用搜索算法搜索，并给出指定长度的序列
    /// user_input: 用户输入的字符串
    /// result_count: 返回的结果，这个值与 `config.show_item_count` 的值保持一致
    /// 返回值：Vec(应用唯一标识符，展示给用户的名字)
    pub async fn update(&self, user_input: &str, result_count: u32) -> Vec<(u64, String)> {
        // 使用核心搜索算法
        let match_results = self.perform_search(user_input, result_count).await;
        // 转换为所需的输出格式
        let program_registry = self.program_registry.read().await;
        let mut result: Vec<(u64, String)> = Vec::new();
        for match_result in match_results {
            let index = *self
                .program_locater
                .get(&match_result.program_guid)
                .expect_programming("程序定位器中未找到程序GUID");
            let program = &program_registry[index];
            result.push((program.program_guid, program.show_name.clone()));
        }
        result
    }

    /// 测试算法
    pub async fn test_search_algorithm(&self, user_input: &str) -> Vec<SearchTestResult> {
        // 使用核心搜索算法
        let total_size = self.get_program_count().await;
        let match_results = self.perform_search(user_input, total_size as u32).await;

        // 转换为详细的测试结果格式
        let mut results: Vec<SearchTestResult> = Vec::new();
        let program_registry = self.program_registry.read().await;
        for match_result in match_results {
            let index = *self
                .program_locater
                .get(&match_result.program_guid)
                .expect_programming("程序定位器中未找到程序GUID");
            let program = &program_registry[index];
            results.push(SearchTestResult {
                program_name: program.show_name.clone(),
                program_keywords: program.search_keywords.join(", "),
                program_path: program.launch_method.get_text(),
                score: match_result.score,
            });
        }

        results
    }

    /// 获取当前程序维护的东西
    pub async fn get_program_infos(&self) -> Vec<(String, bool, f64, String, u64)> {
        let mut result = Vec::new();
        let program_registry = self.program_registry.read().await;
        for item in &*program_registry {
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
    pub async fn launch_program(&self, program_guid: u64, is_admin_required: bool) {
        self.program_launcher
            .launch_program(program_guid, is_admin_required);
    }
    /// 获取程序的图标，返回使用base64编码的png图片
    pub async fn get_icon(&self, program_guid: &u64) -> Vec<u8> {
        let index = *self
            .program_locater
            .get(program_guid)
            .expect_programming("程序定位器中未找到程序GUID");
        let program_registry = self.program_registry.read().await;
        let target_program = &program_registry[index];
        let mut result = self.image_loader.load_image(target_program.clone()).await;
        if let Ok(output) = ImageProcessor::trim_transparent_white_border(result.clone()) {
            result = output;
        }
        result
    }
    /// 获得当前已保存的程序的个数
    pub async fn get_program_count(&self) -> usize {
        let program_registry = self.program_registry.read().await;
        program_registry.len()
    }
    /// 测试搜索算法的时间开销
    pub async fn test_search_algorithm_time(&self) -> (f64, f64, f64) {
        let mut max_time: f64 = 0.0;
        let mut min_time: f64 = 5000.0;
        let mut average_time: f64 = 0.0;
        let program_registry = self.program_registry.read().await;
        let count = (*program_registry)
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
            self.update(&search_text, 5).await;
            let duration = start.elapsed();
            let duration_ms = duration.as_secs_f64() * 1000.0;
            max_time = max_time.max(duration_ms);
            min_time = min_time.min(duration_ms);
            average_time += duration_ms;
        }

        average_time /= count as f64;
        (max_time, min_time, average_time)
    }
    /// 获得加载程序的时间开销
    pub async fn get_program_loader_loading_time(&self) -> f64 {
        self.program_loader.get_loading_time()
    }
    /// 获得搜索关键字
    pub async fn get_search_keywords(&self, show_name: &str) -> Vec<String> {
        self.program_loader.convert_search_keywords(show_name)
    }
    /// 唤醒窗口
    pub async fn activate_target_program(&self, program_guid: u64) -> bool {
        let target_program_index = *self
            .program_locater
            .get(&program_guid)
            .expect_programming("程序定位器中未找到程序GUID");
        let program_registry = self.program_registry.read().await;
        let target_program = program_registry[target_program_index].clone();
        self.window_activator
            .activate_target_program(target_program)
    }
    /// 目标应用程序是不是uwp应用
    pub async fn is_uwp_program(&self, program_guid: u64) -> bool {
        let target_program_index = *self
            .program_locater
            .get(&program_guid)
            .expect_programming("程序定位器中未找到程序GUID");
        let program_registry = self.program_registry.read().await;
        let target_program = program_registry[target_program_index].clone();
        target_program.launch_method.is_uwp()
    }
    /// 打开目标文件所在的文件夹
    pub async fn open_target_folder(&self, program_guid: u64) -> bool {
        self.program_launcher.open_target_folder(program_guid)
    }
    /// 获得最近启动的程序
    pub async fn get_latest_launch_program(&self, program_count: u32) -> Vec<(u64, String)> {
        let latest_launch_program = self
            .program_launcher
            .get_latest_launch_program(program_count);

        let mut results = Vec::new();
        let program_registry = self.program_registry.read().await;
        latest_launch_program.into_iter().for_each(|guid| {
            let index = *self
                .program_locater
                .get(&guid)
                .expect_programming("程序定位器中未找到程序GUID");
            let program_info = program_registry[index].clone();
            results.push((guid, program_info.show_name.clone()));
        });
        results
    }

    async fn perform_search(&self, user_input: &str, result_count: u32) -> Vec<SearchMatchResult> {
        // 预处理用户输入
        let user_input = user_input.to_lowercase();
        let user_input = remove_repeated_space(&user_input);

        let launcher = &self.program_launcher;

        let program_registry = self.program_registry.read().await;
        let search_engine = self.search_engine.read().await;
        // 计算所有程序的匹配分数
        let mut match_scores: Vec<SearchMatchResult> =
            search_engine.perform_search(&user_input, program_registry.as_ref(), launcher);
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
}
