use super::cached_candidate::CachedCandidateData;
use super::candidate_pipeline::CandidatePipeline;
use super::executor_registry::ExecutorRegistry;
use super::search_pipeline::SearchPipeline;
use super::service::PluginService;
use super::types::*;
use crate::core::config::{ConfigEvent, ConfigManager};
use crate::plugin_system::Configurable;
use parking_lot::RwLock;
use std::sync::Arc;
use tracing::{debug, info};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionMode {
    None,
    Plugin(Option<String>),
    Search,
}

pub struct SessionRouter {
    plugin_service: Arc<PluginService>,
    search_pipeline: Arc<RwLock<SearchPipeline>>,
    candidate_pipeline: Arc<RwLock<CandidatePipeline>>,
    cached_candidates: RwLock<CachedCandidateData>,
    current_mode: RwLock<SessionMode>,
    executor_registry: RwLock<ExecutorRegistry>,
    config_manager: RwLock<Option<Arc<ConfigManager>>>,
}

impl SessionRouter {
    pub fn new(plugin_service: Arc<PluginService>) -> Self {
        Self {
            plugin_service,
            search_pipeline: Arc::new(RwLock::new(SearchPipeline::new(None, Vec::new(), 3))),
            candidate_pipeline: Arc::new(RwLock::new(CandidatePipeline::new())),
            cached_candidates: RwLock::new(CachedCandidateData::new()),
            current_mode: RwLock::new(SessionMode::None),
            executor_registry: RwLock::new(ExecutorRegistry::new()),
            config_manager: RwLock::new(None),
        }
    }

    /// 注册一个执行器
    pub fn register_executor(&self, executor: Arc<dyn ActionExecutor>) {
        self.executor_registry
            .write()
            .register(executor)
            .expect("Failed to register executor");
    }

    /// 设置候选管道
    pub fn set_candidate_pipeline(&self, pipeline: CandidatePipeline) {
        *self.candidate_pipeline.write() = pipeline;
    }

    /// 设置搜索管道
    pub fn set_search_pipeline(&self, pipeline: SearchPipeline) {
        *self.search_pipeline.write() = pipeline;
    }

    /// 设置缓存的候选项
    pub fn set_cached_candidates(&self, candidates: CachedCandidateData) {
        *self.cached_candidates.write() = candidates;
    }

    /// 获取缓存的候选项数量
    pub fn get_cached_candidates_count(&self) -> usize {
        self.cached_candidates.read().get_candidates().len()
    }

    pub fn refresh_candidates(&self) {
        let candidates = self.candidate_pipeline.read().collect();
        *self.cached_candidates.write() = candidates;
    }

    pub async fn route_query(&self, trace_id: &str, query: &Query) -> QueryResponse {
        // 生成一个上下文
        let mut ctx = PluginContext::new(trace_id);
        ctx.with_query(query.raw_query.clone());

        let results = self.plugin_service.query(&ctx, query).await;

        // 如果被插件触发了，那么就要进入插件模式，同时记录一下是哪个插件触发了
        if let Some(results) = results {
            // 插件被触发时，会写到上下文中，从而让该函数能被调用
            let plugin_id = ctx.plugin_id.clone();
            *self.current_mode.write() = SessionMode::Plugin(plugin_id.clone());
            return results;
        }
        // 如果插件没有被触发，那么说明当前还是要进入搜索模式

        *self.current_mode.write() = SessionMode::Search;

        let cached_candidate = self.cached_candidates.read();

        let scored_candidates = self
            .search_pipeline
            .as_ref()
            .read()
            .search(&cached_candidate, &query.search_term);

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
                }
            })
            .collect();
        QueryResponse::List { results }
    }

    // 执行一个动作
    // action_id 是用户触发的动作的ID，这个id就是指actions的id，在查询时会被传递到前端，当用户触发一个动作时，前端会把这个id传回来的
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
                let plugin_id =
                    plugin_id.ok_or_else(|| "No plugin in current session".to_string())?;

                ctx.with_plugin_id(plugin_id.clone());
                self.plugin_service
                    .execute_action(&ctx, &plugin_id, action_id, payload)
                    .await
                    .map_err(|e| e.to_string())
            }
            SessionMode::Search => {
                let candidate_id = payload["candidate_id"].as_u64().unwrap_or(0) as CandidateId;
                let query_text = payload["query_text"].as_str().unwrap_or("").to_string();

                let cached_candidate = self.cached_candidates.read();
                let candidate = cached_candidate
                    .get_candidate(candidate_id)
                    .ok_or_else(|| "Candidate not found".to_string())?;

                let exec_ctx = ExecutionContext {
                    target: candidate.target.clone(),
                    display_name: candidate.name.clone(),
                };

                // 执行动作（框架只做透传，不解释 action_id 语义）
                self.executor_registry
                    .read()
                    .execute(&exec_ctx, action_id)
                    .map_err(|e| e.to_string())?;

                // 启动成功后，通知所有 ScoreBooster 记录用户行为
                self.search_pipeline
                    .read()
                    .record(candidate_id, &cached_candidate, &query_text);

                Ok(())
            }
            SessionMode::None => Err("No active session".to_string()),
        }
    }

    pub fn reset_session(&self) {
        *self.current_mode.write() = SessionMode::None;
    }

    pub fn current_mode(&self) -> SessionMode {
        self.current_mode.read().clone()
    }

    /// 设置 ConfigManager 引用并订阅配置变更事件
    pub fn set_config_manager(&self, config_manager: Arc<ConfigManager>) {
        *self.config_manager.write() = Some(config_manager);
    }

    /// 处理配置变更事件。
    /// 根据事件类型执行相应的响应逻辑。
    pub fn handle_config_event(&self, event: &ConfigEvent) {
        match event {
            ConfigEvent::SettingsChanged {
                component_type,
                component_id,
            } => {
                debug!("配置变更事件: {} ({:?})", component_id, component_type);
                match component_type {
                    ComponentType::DataSource | ComponentType::KeywordOptimizer => {
                        // 数据源或关键词优化器变更，需要刷新候选项缓存
                        info!("数据源/关键词优化器配置变更，刷新候选项缓存");
                        self.refresh_candidates();
                    }
                    ComponentType::SearchEngine => {
                        // 搜索引擎变更，需要重建搜索管道
                        // TODO: 在 SearchPipeline 支持动态重建后实现
                        debug!("搜索引擎配置变更");
                    }
                    ComponentType::ScoreBooster => {
                        // 分数增强器变更，需要更新搜索管道
                        // TODO: 在 SearchPipeline 支持动态更新 boosters 后实现
                        debug!("分数增强器配置变更");
                    }
                    ComponentType::Launcher | ComponentType::Plugin | ComponentType::Core => {
                        // Launcher 和 Plugin 不需要 SessionRouter 响应
                        debug!("Launcher/Plugin/Core 配置变更，无需响应");
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
                        // 数据源启用状态变更，需要刷新候选项缓存
                        info!("数据源启用状态变更，刷新候选项缓存");
                        self.refresh_candidates();
                    }
                    ComponentType::SearchEngine | ComponentType::ScoreBooster => {
                        // TODO: 在 Pipeline 支持动态重建后实现
                        debug!("搜索引擎/分数增强器启用状态变更");
                    }
                    ComponentType::Launcher | ComponentType::Plugin | ComponentType::Core => {
                        debug!("Launcher/Plugin/Core 启用状态变更，无需响应");
                    }
                }
            }
            ConfigEvent::Registered { .. } | ConfigEvent::Unregistered { .. } => {
                // 注册/注销事件不需要特殊响应
            }
        }
    }

    /// 根据 component_id 查找已注册的 Configurable 组件。
    /// 参数：component_id - 组件标识符。
    /// 返回：找到则返回组件引用，否则返回 None。
    pub(crate) fn find_configurable(&self, component_id: &str) -> Option<Arc<dyn Configurable>> {
        self.config_manager
            .read()
            .as_ref()
            .and_then(|cm| cm.find_configurable(component_id))
    }

    /// 获取指定组件的配置动作列表。
    /// 参数：component_id - 组件标识符。
    /// 返回：组件支持的配置动作列表，未找到则返回空列表。
    pub fn get_config_actions(&self, component_id: &str) -> Vec<ConfigActionDef> {
        self.find_configurable(component_id)
            .map(|c| c.config_actions())
            .unwrap_or_default()
    }

    /// 执行指定组件的配置动作。
    /// 参数：component_id - 组件标识符，action - 动作标识符。
    /// 返回：动作执行结果（JSON 格式），未找到组件则返回错误。
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
