pub mod builtin_commands;
pub mod config;
pub mod localization_translation;
pub mod pinyin_mapper;
pub mod program_launcher;
pub mod program_loader;
pub mod program_ranker;
pub mod search_engine;
pub mod search_model;
pub mod semantic_backend;
pub mod semantic_manager;
pub mod unit;
pub mod window_activator;
use crate::error::{OptionExt, ResultExt};
use crate::modules::icon_manager::IconManager;
use crate::modules::parameter_resolver::{ParameterResolver, SystemParameterSnapshot};
use crate::modules::program_manager::config::program_manager_config::RuntimeProgramConfig;
use crate::modules::program_manager::search_engine::{SearchEngine, SemanticSearchEngine};
use crate::program_manager::config::program_manager_config::ProgramManagerConfig;
use crate::program_manager::search_engine::TraditionalSearchEngine;
use crate::program_manager::search_model::*;
use crate::program_manager::semantic_manager::SemanticManager;
use crate::program_manager::unit::*;
use config::program_manager_config::PartialProgramManagerConfig;
use dashmap::DashMap;
use lru::LruCache;
use program_launcher::ProgramLauncher;
use program_loader::ProgramLoader;
use program_ranker::ProgramRanker;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{info, warn};
pub use unit::{
    EmbeddingVec, LaunchMethod, LaunchMethodKind, Program, SearchTestResult, SemanticStoreItem,
};
use window_activator::WindowActivator;

/// 语义搜索回退原因（用于 command 层决定提示内容）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackReason {
    None,
    AiDisabled,    // 选择语义，但未启用 AI 特性
    ModelNotReady, // 选择语义，启用 AI，但模型权重未就绪
}

/// 短期搜索结果缓存（统一，基于 LruCache）
type ShortTermSearchResultsCache = Arc<RwLock<Option<LruCache<String, Vec<SearchMatchResult>>>>>;

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
    /// 程序排序器
    program_ranker: Arc<ProgramRanker>,
    /// 当前程序的搜索引擎
    search_engine: Arc<RwLock<Arc<dyn SearchEngine>>>,
    /// 图标获取器
    icon_manager: Arc<IconManager>,
    /// 窗口唤醒器
    window_activator: Arc<WindowActivator>,
    /// 语义生成器
    semantic_manager: Arc<SemanticManager>,
    /// 短期搜索结果缓存（统一，基于 LruCache）
    short_term_result_cache: ShortTermSearchResultsCache,
    /// 当前回退原因
    fallback_reason: Arc<RwLock<FallbackReason>>,
    /// 参数解析器
    parameter_resolver: Arc<ParameterResolver>,
}

/// 内部搜索结果，包含分数和程序ID
#[derive(Debug, Clone)]
pub(crate) struct SearchMatchResult {
    program_guid: u64,
    score_details: ScoreDetails,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProgramDisplayInfo {
    pub name: String,
    pub path: String,
    pub program_guid: u64,
    pub icon_request_json: String,
}

impl ProgramManager {
    /// 初始化，空
    pub fn new(runtime_program_config: RuntimeProgramConfig) -> Self {
        let RuntimeProgramConfig {
            embedding_backend,
            embedding_cache_bytes,
            icon_manager,
            bookmark_loader,
        } = runtime_program_config;

        let semantic_manager = Arc::new(SemanticManager::new(embedding_backend, HashMap::new()));
        let parameter_resolver = Arc::new(ParameterResolver::new());

        let pm = ProgramManager {
            program_registry: Arc::new(RwLock::new(Vec::new())),
            program_loader: Arc::new(ProgramLoader::new(semantic_manager.clone())),
            program_launcher: Arc::new(ProgramLauncher::new()),
            program_ranker: Arc::new(ProgramRanker::new()),
            search_engine: Arc::new(RwLock::new(Arc::new(TraditionalSearchEngine::default()))),
            icon_manager,
            program_locater: Arc::new(DashMap::new()),
            window_activator: Arc::new(WindowActivator::new()),
            semantic_manager,
            short_term_result_cache: Arc::new(RwLock::new(None)),
            fallback_reason: Arc::new(RwLock::new(FallbackReason::None)),
            parameter_resolver,
        };
        if pm
            .semantic_manager
            .load_embeddings_cache_from_bytes(embedding_cache_bytes.as_deref())
        {
            info!("已从持久化缓存加载程序embeddings");
        }

        // 设置书签加载器（在初始化时立即设置，而不是延迟设置）
        pm.program_loader.set_bookmark_loader(bookmark_loader);

        pm
    }

    pub async fn get_runtime_data(&self) -> ProgramManagerRuntimeData {
        // 这里我认为，semantic_store 是一个 HashMap<String, SemanticStoreItem>，而SemanticStoreItem是一个内部的类，它最好不要被外部的信息所接触
        // 所以由ProgramManager来管理其实例化
        // 而PartialProgramManagerConfig本身就是一个用于与外部通信的结构体，所以可以直接返回
        let semantic_store = self.semantic_manager.get_runtime_data();
        let semantic_store_str = serde_json::to_string_pretty(&semantic_store)
            .expect_programming("该结构体在格式化时不应该出错");

        // 导出语义缓存字节（无后端时返回空向量）
        let semantic_cache_bytes: Vec<u8> =
            self.semantic_manager.export_embeddings_cache_to_bytes();

        ProgramManagerRuntimeData {
            semantic_store_str,
            runtime_data: PartialProgramManagerConfig {
                ranker: Some(self.program_ranker.get_runtime_data()),
                loader: None,
                search_model: None,
                enable_lru_search_cache: None,
                search_cache_capacity: None,
            },
            semantic_cache_bytes,
        }
    }

    /// 使用配置信息初始化自身与子模块
    pub async fn load_from_config(
        &self,
        config: Arc<ProgramManagerConfig>,
        semantic_store: Option<String>,
    ) {
        let program_loader_config = &config.get_loader_config();
        let program_ranker_config = &config.get_ranker_config();
        // 先使用semantic_store初始化semantic_manager
        // 这样使用program_loader就可以通过semantic_manager来得到不同的语义描述
        let mut semantic_store = serde_json::from_str::<HashMap<String, SemanticStoreItem>>(
            &semantic_store.unwrap_or_else(|| "{}".to_string()),
        )
        .unwrap_or_default();

        self.semantic_manager
            .update_semantic_store(semantic_store.clone());

        self.program_loader.load_from_config(program_loader_config);

        // 根据搜索模型决定是否生成embedding
        let mut search_config = config.get_search_model_config();
        let has_backend = self.semantic_manager.has_backend();
        let enable_embeddings = has_backend && !search_config.is_traditional_search();
        self.program_loader
            .set_compute_embeddings(enable_embeddings);

        // 加载程序数据
        let new_programs = self.program_loader.load_program();

        // 之后再使用program_loader加载出来的程序再一次更新semantic_manager，因为这一次加载出来的程序可能会有新的程序
        // 更新一下semantic_store，将所有不在semantic_store中的程序添加进去，描述为空
        new_programs.iter().for_each(|program| {
            let key = program.launch_method.get_text();
            semantic_store
                .entry(key)
                .or_insert_with(|| SemanticStoreItem::new(program.clone()));
        });

        self.semantic_manager.update_semantic_store(semantic_store);

        // 清空并更新程序注册表
        let mut program_registry = self.program_registry.write().await;
        program_registry.clear();
        *program_registry = new_programs;

        let programs_to_register: Vec<(u64, LaunchMethod)> = program_registry
            .iter()
            .map(|program| (program.program_guid, program.launch_method.clone()))
            .collect();

        // 加载配置和注册程序到 Ranker
        self.program_ranker
            .load_and_register_programs(program_ranker_config, &programs_to_register);

        // 更新定位器
        self.program_locater.clear();
        for (index, program) in program_registry.iter().enumerate() {
            self.program_locater.insert(program.program_guid, index);
        }

        let is_traditional_search = search_config.is_traditional_search();
        // 语义后端可用性（AI开关+模型权重就绪）
        let backend_ready = self.semantic_manager.is_backend_ready();

        // 更新回退原因
        {
            let mut reason = self.fallback_reason.write().await;
            *reason = if is_traditional_search {
                FallbackReason::None
            } else if !has_backend {
                FallbackReason::AiDisabled
            } else if !backend_ready {
                FallbackReason::ModelNotReady
            } else {
                FallbackReason::None
            };
        }

        let search_engine: Arc<dyn SearchEngine> =
            if !has_backend || !backend_ready || is_traditional_search {
                if !is_traditional_search {
                    search_config = Arc::new(SearchModelConfig::default());
                }
                let new_search_model = SearchModelFactory::create_scorer(search_config.clone());
                Arc::new(TraditionalSearchEngine::new(Arc::new(new_search_model)))
            } else {
                Arc::new(SemanticSearchEngine::new(self.semantic_manager.clone()))
            };

        let mut search_engine_lock = self.search_engine.write().await;
        *search_engine_lock = search_engine;

        if has_backend && (is_traditional_search || !backend_ready) {
            self.semantic_manager.release_backend_resources();
        }

        // 根据配置更新短期搜索缓存（启用时刷新实例，禁用时清空）
        let enable_cache = config.is_lru_search_cache_enabled();
        let capacity = config.get_search_cache_capacity();
        let mut cache_guard = self.short_term_result_cache.write().await;
        if enable_cache {
            let normalized_capacity = capacity.max(1);
            let capacity = NonZeroUsize::new(normalized_capacity)
                .expect("normalized_capacity should be non-zero");
            *cache_guard = Some(LruCache::new(capacity));
        } else {
            *cache_guard = None;
        }
    }

    /// 获取当前回退原因
    pub async fn get_fallback_reason(&self) -> FallbackReason {
        *self.fallback_reason.read().await
    }

    fn get_program_index(&self, program_guid: u64) -> Option<usize> {
        self.program_locater
            .get(&program_guid)
            .map(|entry| *entry.value())
    }

    pub async fn get_program_by_guid(&self, program_guid: u64) -> Option<Arc<Program>> {
        let index = self.get_program_index(program_guid)?;
        let program_registry = self.program_registry.read().await;
        program_registry.get(index).cloned()
    }

    /// 获取参数解析器的引用
    pub fn get_parameter_resolver(&self) -> Arc<ParameterResolver> {
        self.parameter_resolver.clone()
    }

    /// 获取指定程序的启动模板及占位符信息
    pub async fn get_launch_template_info(
        &self,
        program_guid: u64,
    ) -> Option<(String, LaunchMethodKind, usize, String)> {
        let program = self.get_program_by_guid(program_guid).await?;
        let launch_method = program.launch_method.clone();
        let template = launch_method.get_text();
        let kind = launch_method.kind();
        // 使用新的参数解析器统计用户参数数量
        let user_param_count = launch_method.user_parameter_count(&self.parameter_resolver);
        Some((template, kind, user_param_count, program.show_name.clone()))
    }

    /// 使用用户提供的参数填充模板生成新的启动方式
    pub async fn build_launch_method_with_args(
        &self,
        program_guid: u64,
        args: &[String],
        snapshot: &SystemParameterSnapshot,
    ) -> Result<LaunchMethod, String> {
        let program = self
            .get_program_by_guid(program_guid)
            .await
            .ok_or_else(|| format!("Program GUID {} not found", program_guid))?;
        let launch_method = program.launch_method.clone();
        // 使用新的参数解析器
        launch_method.fill_placeholders_with_resolver(args, snapshot, &self.parameter_resolver)
    }

    /// 使用搜索算法搜索，并给出指定长度的序列
    /// user_input: 用户输入的字符串
    /// result_count: 返回的结果，这个值与 `config.show_item_count` 的值保持一致
    /// 返回值：Vec(应用唯一标识符，展示给用户的名字, 启动命令文本)
    pub async fn update(&self, user_input: &str, result_count: u32) -> Vec<(u64, String, String)> {
        // 使用核心搜索算法
        let match_results = self.perform_search(user_input, result_count).await;
        // 转换为所需的输出格式
        let program_registry = self.program_registry.read().await;
        let mut result: Vec<(u64, String, String)> = Vec::new();
        for match_result in match_results {
            let index = *self
                .program_locater
                .get(&match_result.program_guid)
                .expect_programming("程序定位器中未找到程序GUID");
            let program = &program_registry[index];
            result.push((
                program.program_guid,
                program.show_name.clone(),
                program.launch_method.get_text(),
            ));
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
                score_details: match_result.score_details,
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
                self.program_ranker
                    .program_history_launch_time(item.program_guid),
            ));
        }
        result
    }

    /// 记录程序使用（总是记录一次启动使用），并在查询非空时记录查询关联
    pub fn record_query_launch(&self, query: &str, program_guid: u64) {
        // 记录使用意图（提升排序）
        self.program_ranker.record_launch(program_guid);
        // 仅当查询非空时记录关联关系，避免空查询污染关联映射
        if !query.trim().is_empty() {
            self.program_ranker.record_query_launch(query, program_guid);
        }
    }

    /// 启动一个程序
    pub async fn launch_program(
        &self,
        program_guid: u64,
        is_admin_required: bool,
        override_method: Option<LaunchMethod>,
    ) {
        // 获取程序的 launch_method
        let program = self.get_program_by_guid(program_guid).await;
        if program.is_none() {
            warn!("Program with GUID {} not found", program_guid);
            return;
        }
        let program = program.unwrap();

        // 使用 override_method 或程序自己的 launch_method
        let launch_method = override_method.as_ref().unwrap_or(&program.launch_method);

        // 启动程序
        // 因为不管有没有成功，用户都是想启动这个的程序的，所以要考虑到用户的这个意愿
        self.program_launcher
            .launch_program(launch_method, is_admin_required);
    }
    /// 获取程序的图标，返回使用base64编码的png图片
    pub async fn get_icon(&self, program_guid: &u64) -> Vec<u8> {
        let index = *self
            .program_locater
            .get(program_guid)
            .expect_programming(&format!("程序定位器中未找到程序GUID:{}", program_guid));
        let program_registry = self.program_registry.read().await;
        let target_program = &program_registry[index];

        self.icon_manager
            .get_icon(target_program.icon_request.clone())
            .await
    }
    /// 获得已保存的程序的个数
    pub async fn get_program_count(&self) -> usize {
        let program_registry = self.program_registry.read().await;
        program_registry.len()
    }
    /// 获得程序是否是 URL 的列表
    pub async fn get_program_is_url_list(&self) -> Vec<bool> {
        let program_registry = self.program_registry.read().await;
        program_registry
            .iter()
            .map(|program| matches!(program.launch_method, LaunchMethod::Url(_)))
            .collect()
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
        let program = self.get_program_by_guid(program_guid).await;
        if program.is_none() {
            warn!("Program with GUID {} not found", program_guid);
            return false;
        }
        let program = program.unwrap();
        self.program_launcher
            .open_target_folder(&program.launch_method)
    }
    /// 获得最近启动的程序
    pub async fn get_latest_launch_program(
        &self,
        program_count: u32,
    ) -> Vec<(u64, String, String)> {
        let latest_launch_program = self.program_ranker.get_latest_launch_program(program_count);

        let mut results = Vec::new();
        let program_registry = self.program_registry.read().await;
        latest_launch_program.into_iter().for_each(|guid| {
            let index = *self
                .program_locater
                .get(&guid)
                .expect_programming("程序定位器中未找到程序GUID");
            let program_info = program_registry[index].clone();
            results.push((
                guid,
                program_info.show_name.clone(),
                program_info.launch_method.get_text(),
            ));
        });
        results
    }

    /// 轻量级搜索程序（用于设置界面等）
    /// 当 load_all 为 true 时，返回所有匹配的程序；否则限制返回 30 条
    pub async fn search_programs_lightweight(
        &self,
        keyword: &str,
        load_all: bool,
    ) -> Vec<ProgramDisplayInfo> {
        let registry = self.program_registry.read().await;
        let keyword = keyword.to_lowercase();

        let iter = registry
            .iter()
            .filter(|program| program.show_name.to_lowercase().contains(&keyword))
            .map(|program| ProgramDisplayInfo {
                name: program.show_name.clone(),
                path: program.launch_method.get_text().clone(),
                program_guid: program.program_guid,
                icon_request_json: serde_json::to_string(&program.icon_request).unwrap_or_default(),
            });

        if load_all {
            iter.collect()
        } else {
            iter.take(30).collect()
        }
    }

    async fn perform_search(&self, user_input: &str, result_count: u32) -> Vec<SearchMatchResult> {
        // 预处理用户输入
        let user_input = user_input.to_lowercase();
        let user_input = remove_repeated_space(&user_input);

        // 统一短期缓存命中直接返回
        if let Some(cached) = {
            let mut cache_guard = self.short_term_result_cache.write().await;
            cache_guard
                .as_mut()
                .and_then(|cache| cache.get(&user_input).cloned())
        } {
            return cached.into_iter().take(result_count as usize).collect();
        }

        let ranker = &self.program_ranker;

        let program_registry = self.program_registry.read().await;
        let search_engine = self.search_engine.read().await;
        // 计算所有程序的匹配分数
        let mut match_scores: Vec<SearchMatchResult> =
            search_engine.perform_search(&user_input, program_registry.as_ref(), ranker);
        // 按分数降序排序
        match_scores.sort_by(|a, b| {
            b.score_details
                .final_score
                .partial_cmp(&a.score_details.final_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // 只保留需要的数量
        match_scores.truncate(result_count as usize);

        // 写入短期缓存
        if let Some(cache) = self.short_term_result_cache.write().await.as_mut() {
            cache.put(user_input.clone(), match_scores.clone());
        }

        match_scores
    }
}
