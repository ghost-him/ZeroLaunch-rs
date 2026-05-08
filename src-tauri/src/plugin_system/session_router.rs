use super::cached_candidate::CachedCandidateData;
use super::candidate_pipeline::CandidatePipeline;
use super::executor_registry::ExecutorRegistry;
use super::search_pipeline::SearchPipeline;
use super::service::PluginService;
use super::types::*;
use crate::core::config::{ConfigEvent, ConfigManager};
use crate::plugin_system::Configurable;
use crate::sdk::HostApi;
use crate::sdk::ParameterSnapshot;
use parking_lot::{Mutex, RwLock};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionMode {
    None,
    Plugin(String),
    Search,
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

    pub async fn refresh_candidates(&self) {
        let pipeline = self.candidate_pipeline.read().await;
        let candidates = pipeline.collect().await;
        *self.cached_candidates.write() = candidates;
    }

    pub async fn route_query(&self, trace_id: &str, query: &Query) -> QueryResponse {
        let mut ctx = PluginContext::new(trace_id);
        ctx.with_query(query.raw_query.clone());

        let results = self.plugin_service.query(&ctx, query).await;

        if let Some((plugin_id, results)) = results {
            *self.current_mode.write() = SessionMode::Plugin(plugin_id);
            return results;
        }

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

        let results = scored_candidates
            .into_iter()
            .map(|candidate| {
                let search_candidate = cached_candidate
                    .get_candidate(candidate.candidate_id)
                    .unwrap();

                let actions = self
                    .executor_registry
                    .read()
                    .get_actions(search_candidate.target.target_type());

                ListItem {
                    id: search_candidate.id,
                    title: search_candidate.name.clone(),
                    subtitle: search_candidate.name.clone(),
                    icon: search_candidate.icon.clone(),
                    score: candidate.score,
                    actions,
                    target_type: search_candidate.target.target_type().as_str().to_string(),
                }
            })
            .collect();
        QueryResponse::List { results }
    }

    pub async fn route_confirm(
        &self,
        trace_id: &str,
        action_id: &str,
        payload: serde_json::Value,
    ) -> Result<(), String> {
        let mode = self.current_mode.read().clone();
        let mut ctx = PluginContext::new(trace_id);

        match mode {
            SessionMode::Plugin(plugin_id) => {
                ctx.with_plugin_id(plugin_id.clone());
                self.plugin_service
                    .execute_action(&ctx, &plugin_id, action_id, payload)
                    .await
                    .map_err(|e| e.to_string())
            }
            SessionMode::Search => {
                let candidate_id = payload["candidate_id"]
                    .as_u64()
                    .ok_or_else(|| "Missing or invalid candidate_id in payload".to_string())?
                    as CandidateId;
                let query_text = payload["query_text"].as_str().unwrap_or("").to_string();

                let user_args: Vec<String> = payload
                    .get("user_args")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();

                // Build ExecutionContext and record in a block — guard must be dropped before .await
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
                        pipeline.record(candidate_id, &cached_candidate, &query_text);
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
                    Ok(()) => {}
                    Err(ExecutionError::ActivationFailed { fallback_action }) => {
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
                    Err(e) => return Err(e.to_string()),
                }

                Ok(())
            }
            SessionMode::None => Err("No active session".to_string()),
        }
    }

    pub fn reset_session(&self) {
        *self.current_mode.write() = SessionMode::None;
        *self.parameter_snapshot.lock() = ParameterSnapshot::empty();
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
    fn rebuild_search_pipeline(&self) {
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

    pub(crate) fn find_configurable(&self, component_id: &str) -> Option<Arc<dyn Configurable>> {
        self.config_manager
            .read()
            .as_ref()
            .and_then(|cm| cm.find_configurable(component_id))
    }

    pub fn get_config_actions(&self, component_id: &str) -> Vec<ConfigActionDef> {
        self.find_configurable(component_id)
            .map(|c| c.config_actions())
            .unwrap_or_default()
    }

    pub fn execute_config_action(
        &self,
        component_id: &str,
        action: &str,
    ) -> Result<serde_json::Value, String> {
        self.find_configurable(component_id)
            .ok_or_else(|| format!("Component not found: {}", component_id))?
            .execute_config_action(action)
    }
}
