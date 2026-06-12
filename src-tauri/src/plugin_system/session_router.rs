use super::cached_candidate::CachedCandidateData;
use super::candidate_pipeline::CandidatePipeline;
use super::executor_registry::ExecutorRegistry;
use super::search_pipeline::SearchPipeline;
use super::service::PluginService;
use super::types::*;
use crate::core::config::{ConfigEvent, ConfigManager};
use crate::sdk::HostApi;
use parking_lot::{Mutex, RwLock};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info};
use zerolaunch_plugin_api::services::parameter::template_parser::{Placeholder, TemplateParser};
use zerolaunch_plugin_api::services::ParameterSnapshot;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionMode {
    /// 空闲状态
    None,
    /// 普通搜索模式
    Search,
    /// 行内参数输入模式：精确匹配触发词+空格后，用户在搜索栏内直接输入参数
    InlineParam {
        candidate_id: CandidateId,
        trigger_keyword: String,
    },
    /// 参数面板模式：用户按 Enter 后弹出参数面板，逐个填写
    ParamPanel { candidate_id: CandidateId },
    /// 行内插件模式：插件保留搜索栏，控制结果区域（如计算器）
    InlinePlugin(String),
    /// 全页面插件模式：插件接管整个窗口，管理所有按键
    FullPagePlugin(String),
}

impl SessionMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            SessionMode::None => "none",
            SessionMode::Search => "search",
            SessionMode::InlineParam { .. } => "inline_param",
            SessionMode::ParamPanel { .. } => "param_panel",
            SessionMode::InlinePlugin(_) => "inline_plugin",
            SessionMode::FullPagePlugin(_) => "full_page_plugin",
        }
    }

    /// 是否为插件模式（行内或全页面）
    pub fn is_plugin_mode(&self) -> bool {
        matches!(
            self,
            SessionMode::InlinePlugin(_) | SessionMode::FullPagePlugin(_)
        )
    }
}

pub struct SessionRouter {
    plugin_service: Arc<PluginService>,
    search_pipeline: Arc<RwLock<Option<SearchPipeline>>>,
    candidate_pipeline: Arc<tokio::sync::RwLock<CandidatePipeline>>,
    cached_candidates: RwLock<CachedCandidateData>,
    current_mode: RwLock<SessionMode>,
    executor_registry: RwLock<ExecutorRegistry>,
    config_manager: RwLock<Option<Arc<ConfigManager>>>,
    /// HostApi 引用，用于捕获系统参数快照
    host_api: RwLock<Option<Arc<HostApi>>>,
    /// 当前会话的系统参数快照
    /// 在唤醒搜索栏时捕获，执行动作时消费
    parameter_snapshot: Mutex<ParameterSnapshot>,
    /// 搜索引擎注册表（按 component_id 索引），用于动态重建管道
    search_engines: RwLock<HashMap<String, Arc<dyn SearchEngine>>>,
    /// 分数增强器注册表（按 component_id 索引），用于动态重建管道
    score_boosters: RwLock<HashMap<String, Arc<dyn ScoreBooster>>>,
    /// 上次构建管道时的 top_k 值
    last_top_k: RwLock<usize>,
}

impl SessionRouter {
    pub fn new(plugin_service: Arc<PluginService>) -> Self {
        Self {
            plugin_service,
            search_pipeline: Arc::new(RwLock::new(None)),
            candidate_pipeline: Arc::new(tokio::sync::RwLock::new(CandidatePipeline::new())),
            cached_candidates: RwLock::new(CachedCandidateData::new()),
            current_mode: RwLock::new(SessionMode::None),
            executor_registry: RwLock::new(ExecutorRegistry::new()),
            config_manager: RwLock::new(None),
            host_api: RwLock::new(None),
            parameter_snapshot: Mutex::new(ParameterSnapshot::empty()),
            search_engines: RwLock::new(HashMap::new()),
            score_boosters: RwLock::new(HashMap::new()),
            last_top_k: RwLock::new(10),
        }
    }

    /// 注册一个搜索引擎引用，用于配置变更时动态重建管道
    pub fn register_search_engine(&self, engine: Arc<dyn SearchEngine>) {
        self.search_engines
            .write()
            .insert(engine.component_id().to_string(), engine);
    }

    /// 注册一个分数增强器引用，用于配置变更时动态重建管道
    pub fn register_score_booster(&self, booster: Arc<dyn ScoreBooster>) {
        self.score_boosters
            .write()
            .insert(booster.component_id().to_string(), booster);
    }

    /// 注册一个执行器
    pub fn register_executor(&self, executor: Arc<dyn ActionExecutor>) {
        self.executor_registry
            .write()
            .register(executor)
            .expect("Failed to register executor");
    }

    /// 注册一个第三方数据源（供 plugin_loader 调用）
    pub async fn register_data_source(&self, source: Arc<dyn DataSource>) {
        self.candidate_pipeline.write().await.add_source(source);
    }

    /// 注册一个第三方插件（供 plugin_loader 调用）
    pub fn register_remote_plugin(&self, plugin: Arc<dyn Plugin>) {
        self.plugin_service.register(plugin);
    }

    /// 注销一个数据源（按 component_id）
    pub async fn unregister_data_source(&self, component_id: &str) {
        self.candidate_pipeline
            .write()
            .await
            .remove_source(component_id);
    }

    /// 注销一个执行器（按 component_id）
    pub fn unregister_executor(&self, component_id: &str) {
        self.executor_registry.write().unregister(component_id);
    }

    /// 注销一个插件（按 plugin_id）
    pub fn unregister_plugin(&self, plugin_id: &str) {
        self.plugin_service.unregister(plugin_id);
    }

    /// 设置 HostApi 引用
    pub fn set_host_api(&self, host_api: Arc<HostApi>) {
        *self.host_api.write() = Some(host_api);
    }

    /// 设置候选管道
    pub async fn set_candidate_pipeline(&self, pipeline: CandidatePipeline) {
        *self.candidate_pipeline.write().await = pipeline;
    }

    /// 设置搜索管道
    pub fn set_search_pipeline(&self, pipeline: SearchPipeline) {
        *self.last_top_k.write() = pipeline.top_k();
        *self.search_pipeline.write() = Some(pipeline);
    }

    /// 设置缓存的候选项
    pub fn set_cached_candidates(&self, candidates: CachedCandidateData) {
        *self.cached_candidates.write() = candidates;
    }

    /// 获取缓存的候选项数量
    pub fn get_cached_candidates_count(&self) -> usize {
        self.cached_candidates.read().get_candidates().len()
    }

    /// 获取所有缓存的候选项克隆
    pub fn get_cached_candidates(&self) -> Vec<SearchCandidate> {
        self.cached_candidates.read().get_candidates().to_vec()
    }

    /// 根据 ID 获取单个缓存的候选项
    pub fn get_cached_candidate_by_id(&self, id: CandidateId) -> Option<SearchCandidate> {
        self.cached_candidates.read().get_candidate(id).cloned()
    }

    /// 获取候选项的快照（计数 + 数据），单次锁获取保证一致性
    pub fn get_candidates_snapshot(&self) -> (usize, Vec<SearchCandidate>) {
        let guard = self.cached_candidates.read();
        let candidates = guard.get_candidates();
        (candidates.len(), candidates.to_vec())
    }

    pub async fn refresh_candidates(&self) {
        let pipeline = self.candidate_pipeline.read().await;
        let candidates = pipeline.collect().await;
        *self.cached_candidates.write() = candidates;
    }

    pub async fn route_query(&self, trace_id: &str, query: &Query) -> QueryResponse {
        let mut ctx = PluginContext::new(trace_id);
        ctx.with_query(query.raw_query.clone());

        // 这里的查询路由逻辑是：优先让插件处理查询（如果匹配），否则走内置搜索管道。
        let results = self.plugin_service.query(&ctx, query).await;

        if let Some((plugin_id, results)) = results {
            // 根据插件的 keep_search_bar 选择行内或全页面模式
            let mode = match &results {
                QueryResponse::CustomPanel {
                    keep_search_bar, ..
                } => {
                    if *keep_search_bar {
                        SessionMode::InlinePlugin(plugin_id)
                    } else {
                        SessionMode::FullPagePlugin(plugin_id)
                    }
                }
                _ => SessionMode::InlinePlugin(plugin_id),
            };
            *self.current_mode.write() = mode;
            return results;
        }

        // 任何新查询隐式重置会话模式为 Search，
        // 这是前端 exitInlineParamMode / exitParamPanelMode / exitPluginMode 通过 doQuery('') 退出模式的契约基础。
        *self.current_mode.write() = SessionMode::Search;

        let cached_candidate = self.cached_candidates.read();

        let pipeline_guard = self.search_pipeline.read();
        let pipeline = match pipeline_guard.as_ref() {
            Some(p) => p,
            None => {
                tracing::warn!("SearchPipeline 未初始化，返回空结果");
                return QueryResponse::Empty;
            }
        };
        let scored_candidates = pipeline.search(&cached_candidate, &query.search_term);

        // 检测行内参数模式入口：查询以空格结尾 + 去掉空格后精确匹配某候选项的触发关键词。
        // 在 ListItem 映射之前检查，避免匹配时废弃已映射的结果。
        if query.raw_query.ends_with(' ') {
            let trimmed = query.search_term.trim();
            for candidate in &scored_candidates {
                let Some(sc) = cached_candidate.get_candidate(candidate.candidate_id) else {
                    tracing::warn!(
                        "Inline param check: candidate {} not found in cache, skipping",
                        candidate.candidate_id
                    );
                    continue;
                };
                let user_arg_count = TemplateParser::count_user_args(sc.target.payload());
                if user_arg_count > 0
                    && sc
                        .trigger_keywords
                        .iter()
                        .any(|kw| kw.to_lowercase() == trimmed)
                {
                    *self.current_mode.write() = SessionMode::InlineParam {
                        candidate_id: sc.id,
                        trigger_keyword: trimmed.to_string(),
                    };
                    return QueryResponse::InlineParam {
                        candidate_id: sc.id,
                        trigger_keyword: trimmed.to_string(),
                        user_arg_count,
                    };
                }
            }
        }

        let results: Vec<ListItem> = scored_candidates
            .into_iter()
            .filter_map(|candidate| {
                let Some(search_candidate) = cached_candidate.get_candidate(candidate.candidate_id)
                else {
                    tracing::warn!(
                        "List mapping: candidate {} not found in cache, skipping",
                        candidate.candidate_id
                    );
                    return None;
                };

                let actions = self
                    .executor_registry
                    .read()
                    .get_actions(search_candidate.target.target_type());

                let template_str = search_candidate.target.payload();
                let placeholders = TemplateParser::parse(template_str);
                let user_arg_count = placeholders
                    .iter()
                    .filter(|p| matches!(p, Placeholder::UserArg))
                    .count();
                let has_system_params = placeholders
                    .iter()
                    .any(|p| matches!(p, Placeholder::System(_)));
                let trigger_keywords = search_candidate.trigger_keywords.clone();

                Some(ListItem {
                    id: search_candidate.id,
                    title: search_candidate.name.clone(),
                    subtitle: search_candidate.name.clone(),
                    icon: search_candidate.icon.clone(),
                    score: candidate.score,
                    actions,
                    target_type: search_candidate.target.target_type().as_str().to_string(),
                    user_arg_count,
                    has_system_params,
                    trigger_keywords,
                })
            })
            .collect();

        QueryResponse::List { results }
    }

    pub async fn route_confirm(
        &self,
        trace_id: &str,
        action_id: &str,
        payload: serde_json::Value,
    ) -> Result<ConfirmResult, String> {
        let mode = self.current_mode.read().clone();
        let mut ctx = PluginContext::new(trace_id);

        match mode {
            SessionMode::InlinePlugin(ref plugin_id)
            | SessionMode::FullPagePlugin(ref plugin_id) => {
                ctx.with_plugin_id(plugin_id.clone());
                self.plugin_service
                    .execute_action(&ctx, plugin_id, action_id, payload)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(ConfirmResult::Executed)
            }
            SessionMode::InlineParam { candidate_id, .. } => {
                let user_args = Self::extract_user_args(&payload);
                let query_text = payload
                    .get("query_text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                self.execute_candidate(candidate_id, action_id, user_args, query_text)
                    .await?;
                Ok(ConfirmResult::Executed)
            }
            SessionMode::ParamPanel { candidate_id } => {
                let user_args = Self::extract_user_args(&payload);
                let query_text = payload
                    .get("query_text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                self.execute_candidate(candidate_id, action_id, user_args, query_text)
                    .await?;
                Ok(ConfirmResult::Executed)
            }
            SessionMode::Search => {
                let candidate_id = payload["candidate_id"]
                    .as_u64()
                    .ok_or_else(|| "Missing or invalid candidate_id in payload".to_string())?
                    as CandidateId;
                let query_text = payload["query_text"].as_str().unwrap_or("").to_string();
                let user_args = Self::extract_user_args(&payload);

                // 候选项需要参数但用户未提供 → 引导进入参数面板
                let user_arg_count = {
                    let cc = self.cached_candidates.read();
                    cc.get_candidate(candidate_id)
                        .map(|c| TemplateParser::count_user_args(c.target.payload()))
                        .unwrap_or(0)
                };
                if user_arg_count > 0 && user_args.is_empty() {
                    *self.current_mode.write() = SessionMode::ParamPanel { candidate_id };
                    return Ok(ConfirmResult::EnterParamPanel {
                        candidate_id,
                        user_arg_count,
                    });
                }

                self.execute_candidate(candidate_id, action_id, user_args, &query_text)
                    .await?;
                Ok(ConfirmResult::Executed)
            }
            SessionMode::None => Err("No active session".to_string()),
        }
    }

    /// 从 payload 中提取 user_args
    fn extract_user_args(payload: &serde_json::Value) -> Vec<String> {
        payload
            .get("user_args")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 统一的候选项执行逻辑，被 InlineParam/ParamPanel/Search 模式共用
    async fn execute_candidate(
        &self,
        candidate_id: CandidateId,
        action_id: &str,
        user_args: Vec<String>,
        query_text: &str,
    ) -> Result<(), String> {
        let exec_ctx = {
            let cached_candidate = self.cached_candidates.read();
            let candidate = cached_candidate
                .get_candidate(candidate_id)
                .ok_or_else(|| "Candidate not found".to_string())?;

            let snapshot = self.parameter_snapshot.lock().clone();

            let exec_ctx = ExecutionContext {
                target: candidate.target.clone(),
                display_name: candidate.name.clone(),
                user_args,
                parameter_snapshot: snapshot,
            };

            if let Some(pipeline) = self.search_pipeline.read().as_ref() {
                pipeline.record(candidate_id, &cached_candidate, query_text);
            }

            exec_ctx
        };
        // All RwLock/Mutex guards dropped here — safe to .await

        let executor = {
            let registry = self.executor_registry.read();
            registry
                .resolve(&exec_ctx, action_id)
                .map_err(|e| e.to_string())?
        };

        match executor.execute(&exec_ctx, action_id).await {
            Ok(()) => Ok(()),
            Err(ExecutionError::ActivationFailed { fallback_action }) => {
                let launch_new = self
                    .config_manager
                    .read()
                    .as_ref()
                    .and_then(|cm| {
                        cm.get_component_setting("window-behavior", "launch_new_on_failure")
                    })
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                if launch_new {
                    let fallback_executor = {
                        let registry = self.executor_registry.read();
                        registry
                            .resolve_fallback(&exec_ctx, &fallback_action)
                            .map_err(|e| e.to_string())?
                    };
                    fallback_executor
                        .execute(&exec_ctx, &fallback_action)
                        .await
                        .map_err(|e| e.to_string())?;
                }
                Ok(())
            }
            Err(e) => Err(e.to_string()),
        }
    }

    /// 参数面板和行内参数模式始终重置；插件模式仅在 `reset_plugins` 为 true 时重置，
    /// 以支持在隐藏/显示间保持插件面板状态。
    /// 返回 true 表示实际执行了重置操作。
    pub fn reset_session(&self, reset_plugins: bool) -> bool {
        let mut mode = self.current_mode.write();
        let should_reset = match &*mode {
            SessionMode::None => false,
            SessionMode::InlinePlugin(_) | SessionMode::FullPagePlugin(_) => reset_plugins,
            SessionMode::Search
            | SessionMode::InlineParam { .. }
            | SessionMode::ParamPanel { .. } => true,
        };
        if should_reset {
            *mode = SessionMode::None;
            *self.parameter_snapshot.lock() = ParameterSnapshot::empty();
        }
        should_reset
    }

    pub fn current_mode(&self) -> SessionMode {
        self.current_mode.read().clone()
    }

    pub fn plugin_service(&self) -> &Arc<PluginService> {
        &self.plugin_service
    }

    pub async fn on_search_bar_wake(&self) -> Result<(), String> {
        let host_api = self
            .host_api
            .read()
            .clone()
            .ok_or_else(|| "HostApi not initialized in SessionRouter".to_string())?;

        let snapshot = host_api.capture_parameter_snapshot().await;
        *self.parameter_snapshot.lock() = snapshot;

        debug!("📸 搜索栏唤醒，系统参数快照已捕获");
        Ok(())
    }

    pub fn set_config_manager(&self, config_manager: Arc<ConfigManager>) {
        *self.config_manager.write() = Some(config_manager);
    }

    /// 动态重建搜索管道。
    /// 根据当前已注册且启用的 SearchEngine 和 ScoreBooster 重建 SearchPipeline。
    pub fn rebuild_search_pipeline(&self) {
        let cm_guard = self.config_manager.read();
        let cm = match cm_guard.as_ref() {
            Some(cm) => cm,
            None => return,
        };

        // 收集启用的搜索引擎（取第一个）
        let engines = self.search_engines.read();
        let enabled_engine = engines
            .values()
            .find(|e| cm.is_enabled(e.component_id()))
            .cloned();

        // 收集启用的分数增强器（保持注册顺序）
        let boosters = self.score_boosters.read();
        let enabled_boosters: Vec<Arc<dyn ScoreBooster>> = boosters
            .values()
            .filter(|b| cm.is_enabled(b.component_id()))
            .cloned()
            .collect();

        if let Some(engine) = enabled_engine {
            let top_k = *self.last_top_k.read();
            let pipeline = SearchPipeline::new(engine, enabled_boosters, top_k);
            *self.search_pipeline.write() = Some(pipeline);
            info!(
                "搜索管道已重建 (搜索引擎: 1, 增强器: {}, top_k: {})",
                boosters.len(),
                top_k
            );
        } else {
            tracing::warn!("没有启用的搜索引擎，无法重建搜索管道");
        }
    }

    /// 处理配置变更事件。
    /// 根据事件类型执行相应的响应逻辑。
    pub async fn handle_config_event(&self, event: &ConfigEvent) {
        match event {
            ConfigEvent::SettingsChanged {
                component_type,
                component_id,
            } => {
                debug!("配置变更事件: {} ({:?})", component_id, component_type);
                match component_type {
                    ComponentType::DataSource | ComponentType::KeywordOptimizer => {
                        info!("数据源/关键词优化器配置变更，刷新候选项缓存");
                        self.refresh_candidates().await;
                    }
                    ComponentType::SearchEngine | ComponentType::ScoreBooster => {
                        info!("搜索引擎/分数增强器配置变更，重建搜索管道");
                        self.rebuild_search_pipeline();
                    }
                    ComponentType::ActionExecutor | ComponentType::Plugin | ComponentType::Core => {
                        debug!("ActionExecutor/Plugin/Core 配置变更，无需响应");
                    }
                }
            }
            ConfigEvent::EnabledChanged {
                component_type,
                component_id,
                enabled,
            } => {
                debug!(
                    "启用状态变更事件: {} ({:?}), enabled={}",
                    component_id, component_type, enabled
                );
                match component_type {
                    ComponentType::DataSource | ComponentType::KeywordOptimizer => {
                        info!("数据源启用状态变更，刷新候选项缓存");
                        self.refresh_candidates().await;
                    }
                    ComponentType::SearchEngine | ComponentType::ScoreBooster => {
                        info!("搜索引擎/分数增强器启用状态变更，重建搜索管道");
                        self.rebuild_search_pipeline();
                    }
                    ComponentType::ActionExecutor | ComponentType::Plugin | ComponentType::Core => {
                        debug!("ActionExecutor/Plugin/Core 启用状态变更，无需响应");
                    }
                }
            }
            ConfigEvent::Registered { .. } | ConfigEvent::Unregistered { .. } => {}
        }
    }
}
