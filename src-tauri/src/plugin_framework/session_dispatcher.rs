//! SessionDispatcher —— 会话调度器（取代 SessionRouter）。
//!
//! 直接内嵌默认搜索与触发式插件的查询/确认逻辑（SessionRouter 式，无流程抽象层）：
//! 触发词路由、默认搜索（行内参数检测、ListItem 构造、参数面板引导）、候选项执行
//! （含 ActivationFailed fallback）、插件 execute_action 转发。
//! 会话系统层保留：代际、session-state 事件、面板动作通道、管道重建。
//!
//! 会话状态仅由 UI 通道维护（channel == Ui）；CLI/调试查询为只读辅助路径，
//! 不改写活动会话、不推送事件（原 SessionRouter 行为保持）。

use dashmap::DashMap;
use dashmap::DashSet;
use parking_lot::{Mutex, RwLock};
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};
use zerolaunch_plugin_api::config::ComponentType;
use zerolaunch_plugin_api::services::parameter::template_parser::{Placeholder, TemplateParser};
use zerolaunch_plugin_api::services::ParameterSnapshot;
use zerolaunch_plugin_api::{
    CachedCandidateData, CandidateId, ExecutionContext, ExecutionError, ListItem, Plugin,
    PluginContext, Query, QueryChannel, QueryResponse, QueryRevisionGate,
};

use super::candidate_pipeline::CandidatePipeline;
use super::component_registry::PluginComponentRegistry;
use super::executor_registry::ExecutorRegistry;
use super::registry::PluginRegistry;
use super::search_pipeline::SearchPipeline;
use super::session_state::{
    ActiveSession, PanelContentAction, PluginPanelContent, PluginPanelInfo, PresentationMode,
    SessionStateEmitter, SessionStateEvent,
};
use crate::core::config::bias_settings::{bias_settings_to_rules, BiasSettings};
use crate::core::config::{ConfigEvent, ConfigManager};
use crate::core::i18n::I18nManager;
use crate::sdk::HostApi;
use crate::utils::collapse_repeated_spaces;

/// 调度器内部错误类型。
/// 仅在 plugin_framework 层内部使用，不暴露到 IPC 边界；
/// 在 commands/ 层通过 From 转换为 BridgeError。
#[derive(Debug)]
pub enum SessionDispatcherError {
    /// 服务未初始化
    NotInitialized(String),
    /// 候选项未找到
    CandidateNotFound(u64),
    /// 请求负载无效
    InvalidPayload(String),
    /// 会话状态无效（含会话代际过期）
    InvalidState(String),
    /// 插件服务执行错误
    PluginError(String),
    /// 执行器错误
    ExecutionError(String),
    /// 常规内部错误
    Internal(String),
}

impl fmt::Display for SessionDispatcherError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionDispatcherError::NotInitialized(msg) => {
                write!(f, "SessionDispatcher 未初始化: {}", msg)
            }
            SessionDispatcherError::CandidateNotFound(id) => {
                write!(f, "候选项未找到: id={}", id)
            }
            SessionDispatcherError::InvalidPayload(msg) => {
                write!(f, "请求负载无效: {}", msg)
            }
            SessionDispatcherError::InvalidState(msg) => {
                write!(f, "会话状态无效: {}", msg)
            }
            SessionDispatcherError::PluginError(msg) => {
                write!(f, "插件执行错误: {}", msg)
            }
            SessionDispatcherError::ExecutionError(msg) => {
                write!(f, "执行器错误: {}", msg)
            }
            SessionDispatcherError::Internal(msg) => write!(f, "内部错误: {}", msg),
        }
    }
}

impl std::error::Error for SessionDispatcherError {}

/// 路由查询结果 —— 响应 + 会话代际 + 会话归属。
///
/// `generation` 随响应下发（双通道代际同步，见设计 §5.4），供前端更新
/// `currentGeneration` 并在后续确认时回传校验；`plugin_id` 供 Inspector 可观测。
#[derive(Debug)]
pub struct RoutedQuery {
    /// 展示响应。
    pub response: QueryResponse,
    /// 路由完成后的会话代际。
    pub generation: u64,
    /// 实际处理本次查询的会话归属（None = 宿主默认搜索）。
    pub plugin_id: Option<String>,
}

/// 确认结局 —— Dispatcher 层语义，核心程序专属（无流程抽象）。
///
/// 仅由 `route_confirm` 返回并经命令层映射为 IPC 响应
/// （`BridgeConfirmResponse` 承担序列化契约，本类型不跨 IPC）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmOutcome {
    /// 动作已执行完成。
    Executed,
    /// 进入参数面板（输入收集形态）：携带参数面板专属载荷
    /// （候选 ID + 参数个数），前端据此构造输入字段，无需依赖列表项。
    EnterParamPanel {
        /// 目标候选项 ID（参数面板收集完成后确认时使用）。
        candidate_id: CandidateId,
        /// 模板参数个数（与后端 TemplateParser 计算同源）。
        user_arg_count: usize,
    },
}

/// 路由确认结果 —— 确认结局 + 会话代际。
///
/// 与 `RoutedQuery` 同模式：Dispatcher 对命令层的返回类型，承载「结局 + 会话元数据」，
/// 不含会话内部语义（子状态由 Dispatcher 自持，IPC 序列化由命令层完成）。
/// 供 `bridge_confirm` 构造 `BridgeConfirmResponse`。
#[derive(Debug)]
pub struct RoutedConfirm {
    /// 确认结局。
    pub outcome: ConfirmOutcome,
    /// 路由完成后的会话代际（进入输入收集面板会递增代际，随响应回传前端单调更新）。
    pub generation: u64,
}

/// 执行错误（确认/执行链路的错误载荷）。
#[derive(Debug, thiserror::Error)]
#[error("执行失败: {0}")]
pub struct ConfirmError(pub String);

/// 确认请求 —— 两条确认路径的显式建模（命令层构造，Dispatcher 消费，统一经 bridge_confirm 通道）：
/// - `Candidate`：宿主确认（默认搜索：列表/行内参数/参数面板执行；插件面板内执行默认动作），全程类型化；
/// - `PluginAction`：插件面板动作（面板按键契约 Custom / GotoPanel），载荷为插件自由 JSON
///   （`execute_action` 的 IPC 契约，宿主不做形状约束）。
#[derive(Debug)]
pub enum ConfirmRequest {
    /// 宿主候选确认：执行候选项（缺参数时引导参数面板）。
    Candidate {
        /// 目标候选项 ID。
        candidate_id: CandidateId,
        /// 动作 ID。
        action_id: String,
        /// 发起确认时的查询文本。
        query_text: String,
        /// 用户参数（行内参数/参数面板场景）。
        user_args: Vec<String>,
        /// 前端最后一次观测到的会话代际。
        generation: u64,
    },
    /// 插件面板动作：自定义能力调用（面板按键契约 Custom / GotoPanel 回插件）。
    PluginAction {
        /// 声明发起动作的插件（Dispatcher 路由时校验归属，须与活动会话一致）。
        plugin_id: String,
        /// 插件动作 ID（插件 `execute_action` 的分支名）。
        action: String,
        /// 插件自定义载荷（自由 JSON）。
        args: serde_json::Value,
        /// 当前会话代际（Dispatcher 路由面板动作时填充）。
        generation: u64,
    },
}

impl ConfirmRequest {
    /// 请求携带的会话代际（两条路径共用，供调度器校验会话归属）。
    pub fn generation(&self) -> u64 {
        match self {
            ConfirmRequest::Candidate { generation, .. }
            | ConfirmRequest::PluginAction { generation, .. } => *generation,
        }
    }
}

/// 默认搜索子状态（InlineParam/ParamPanel 属默认搜索的会话状态）。
/// 行内参数的 trigger_keyword 仅存在于响应契约（QueryResponse::InlineParam），
/// 确认路由只依赖 candidate_id，无需保存触发词。
#[derive(Debug, Clone)]
enum SearchSubState {
    /// 常规搜索。
    Search,
    /// 行内参数输入中（候选已锁定）。
    InlineParam { candidate_id: CandidateId },
    /// 参数面板收集输入中（候选已锁定）。
    ParamPanel { candidate_id: CandidateId },
}

pub struct SessionDispatcher {
    /// 插件注册中心（插件 init 在 bootstrap 完成）。
    plugin_registry: Arc<PluginRegistry>,
    /// 触发词索引：trigger → plugin_id（一个触发词只能绑定一个插件，冲突注册即拒绝）。
    trigger_index: DashMap<String, String>,
    /// 插件级启用状态集合（注册/启停时同步；wake_plugin 启用校验的权威依据——
    /// 禁用插件即使前端热键表残留也不得被唤醒）。DashSet 并发安全，免去外部锁。
    enabled_plugins: DashSet<String>,
    /// 活动会话（权威投影，代际随其写入递增）。
    active_session: RwLock<ActiveSession>,

    // ---- 横切：默认搜索服务（Dispatcher 直接持有，管道重建对查询无感）----
    search_pipeline: Arc<RwLock<Option<SearchPipeline>>>,
    candidate_pipeline: Arc<tokio::sync::RwLock<CandidatePipeline>>,
    cached_candidates: Arc<RwLock<CachedCandidateData>>,
    executor_registry: Arc<RwLock<ExecutorRegistry>>,
    config_manager: Arc<RwLock<Option<Arc<ConfigManager>>>>,
    host_api: RwLock<Option<Arc<HostApi>>>,
    /// 后端翻译服务（查询上下文填充当前语言用；CLI 场景不注入时为空串）。
    i18n: RwLock<Option<Arc<I18nManager>>>,
    /// 默认搜索子状态（行内参数/参数面板）。
    search_state: RwLock<SearchSubState>,
    /// 当前会话的系统参数快照（唤醒时捕获，执行动作时消费）。
    parameter_snapshot: Arc<Mutex<ParameterSnapshot>>,
    /// 插件运行时组件注册中心（管道重建工厂）。
    components: PluginComponentRegistry,
    /// 上次构建管道时的 top_k 值。
    last_top_k: RwLock<usize>,
    /// 最近一次候选项刷新的时间点（所有触发源共用：定时/监控/手动/配置联动）。
    /// None 表示从未刷新过（定时任务应视为超期立即刷新）；
    /// 每次 refresh_candidates 成功更新，供 auto-refresh 周期任务判断是否到达间隔。
    last_refresh: Mutex<Option<Instant>>,

    /// 会话状态推送回调（bootstrap 注入；CLI 无窗口场景不注入）。
    session_emitter: RwLock<Option<SessionStateEmitter>>,
    /// 三通道查询版本计数器（语义见 QueryRevisionGate 注释）。
    ui_query_revision: Arc<AtomicU64>,
    cli_query_revision: Arc<AtomicU64>,
    debug_query_revision: Arc<AtomicU64>,
}

impl SessionDispatcher {
    /// 创建调度器。
    /// 参数：plugin_registry - 插件注册中心（注册/注销/枚举）。
    pub fn new(plugin_registry: Arc<PluginRegistry>) -> Self {
        Self {
            plugin_registry,
            trigger_index: DashMap::new(),
            enabled_plugins: DashSet::new(),
            active_session: RwLock::new(ActiveSession {
                generation: 0,
                plugin_id: None,
                presentation: PresentationMode::None,
            }),
            search_pipeline: Arc::new(RwLock::new(None)),
            candidate_pipeline: Arc::new(tokio::sync::RwLock::new(CandidatePipeline::new())),
            cached_candidates: Arc::new(RwLock::new(CachedCandidateData::new())),
            executor_registry: Arc::new(RwLock::new(ExecutorRegistry::new())),
            config_manager: Arc::new(RwLock::new(None)),
            host_api: RwLock::new(None),
            i18n: RwLock::new(None),
            search_state: RwLock::new(SearchSubState::Search),
            parameter_snapshot: Arc::new(Mutex::new(ParameterSnapshot::empty())),
            components: PluginComponentRegistry::new(),
            last_top_k: RwLock::new(10),
            last_refresh: Mutex::new(None),
            session_emitter: RwLock::new(None),
            ui_query_revision: Arc::new(AtomicU64::new(0)),
            cli_query_revision: Arc::new(AtomicU64::new(0)),
            debug_query_revision: Arc::new(AtomicU64::new(0)),
        }
    }

    // ==================== 注册与装配 ====================

    /// 注册一个执行器。
    pub fn register_executor(&self, executor: Arc<dyn zerolaunch_plugin_api::ActionExecutor>) {
        self.executor_registry
            .write()
            .register(executor)
            .expect("Failed to register executor");
    }

    /// 注销一个执行器（按 component_id）。
    pub fn unregister_executor(&self, component_id: &str) {
        self.executor_registry.write().unregister(component_id);
    }

    /// 注册一个插件（内置/第三方统一入口）：注册服务 + 建立触发词索引。
    /// 触发词冲突时拒绝并记录错误（不覆盖既有绑定）。
    /// `enabled` 为当前持久化启用状态：禁用状态的插件（如用户上次关闭后重启）注册时不写入触发词，
    /// 与运行时 set_plugin_enabled 的「禁用即不路由」语义保持一致。
    /// 注意：这是触发词索引的写入入口之一，注册内置插件时也必须走此方法。
    /// 触发词索引的完整写入路径：register_plugin_with_triggers（注册时按 enabled 建立）、
    /// unregister_plugin（注销时清理）、set_plugin_enabled（启用恢复/禁用清理）。
    pub fn register_plugin_with_triggers(&self, plugin: Arc<dyn Plugin>, enabled: bool) {
        // 先校验全部触发词无冲突，再注册与写入，避免半提交状态。
        let keywords = plugin.metadata().trigger_keywords.clone();
        let conflicts: Vec<&str> = keywords
            .iter()
            .filter(|kw| self.trigger_index.contains_key(*kw))
            .map(|s| s.as_str())
            .collect();
        if !conflicts.is_empty() {
            error!(
                "注册插件 '{}' 失败：触发词冲突 {:?}",
                plugin.metadata().id,
                conflicts
            );
            return;
        }
        self.plugin_registry.register(plugin.clone());
        self.set_plugin_enabled_state(&plugin.metadata().id, enabled);
        if enabled {
            self.try_insert_trigger_keywords(&plugin.metadata().id, &keywords);
        } else {
            info!(
                "插件 '{}' 处于禁用状态，跳过触发词写入（启用时恢复）",
                plugin.metadata().id
            );
        }
    }

    /// 写入触发词：逐词校验，被其他插件占用的词跳过并记录错误（不覆盖既有绑定）。
    /// 用于 set_plugin_enabled 的启用恢复——注册与启停两条路径共用同一冲突规则。
    fn try_insert_trigger_keywords(&self, plugin_id: &str, keywords: &[String]) {
        for kw in keywords {
            if let Some(owner) = self.trigger_index.get(kw) {
                if owner.as_str() != plugin_id {
                    error!(
                        "恢复触发词 '{}' 冲突：已被插件 '{}' 占用，跳过（插件 '{}' 的该词不恢复）",
                        kw,
                        owner.as_str(),
                        plugin_id
                    );
                    continue;
                }
            }
            self.trigger_index.insert(kw.clone(), plugin_id.to_string());
        }
    }

    /// 移除插件的全部触发词路由；活动会话属于该插件时先执行会话重置。
    /// 注销与禁用共用：两者语义都是「该插件不再可路由」。
    fn remove_plugin_routes(&self, plugin_id: &str) {
        self.trigger_index.retain(|_, v| v != plugin_id);
        if self.active_session.read().plugin_id.as_deref() == Some(plugin_id) {
            self.reset_session(true);
        }
    }

    /// 注销一个插件：移除注册 + 触发词路由；活动会话属于该插件时先执行会话重置。
    pub fn unregister_plugin(&self, plugin_id: &str) {
        self.plugin_registry.unregister(plugin_id);
        self.enabled_plugins.remove(plugin_id);
        self.remove_plugin_routes(plugin_id);
    }

    /// 写入插件级启用状态（触发词索引与 wake_plugin 启用校验共用同一状态源）。
    fn set_plugin_enabled_state(&self, plugin_id: &str, enabled: bool) {
        if enabled {
            self.enabled_plugins.insert(plugin_id.to_string());
        } else {
            self.enabled_plugins.remove(plugin_id);
        }
    }

    /// 查询插件级启用状态（wake_plugin 启用校验；禁用插件不可被热键唤醒）。
    pub fn is_plugin_enabled(&self, plugin_id: &str) -> bool {
        self.enabled_plugins.contains(plugin_id)
    }

    /// 插件启用状态变更时同步触发词索引：
    /// 禁用 → 移除该插件全部触发词（搜索不再路由到它）；启用 → 恢复注册时的触发词
    /// （逐词冲突检查：期间被其他插件占用的词跳过并记录错误，不覆盖既有绑定）。
    /// 插件实例仍保留在 registry 中，不销毁（区别于 unregister_plugin）。
    pub fn set_plugin_enabled(&self, plugin_id: &str, enabled: bool) {
        self.set_plugin_enabled_state(plugin_id, enabled);
        if enabled {
            let Some(plugin) = self.plugin_registry.get(plugin_id) else {
                debug!("启用插件 {} 不在注册表中，跳过触发词恢复", plugin_id);
                return;
            };
            let keywords = plugin.metadata().trigger_keywords.clone();
            self.try_insert_trigger_keywords(plugin_id, &keywords);
        } else {
            self.remove_plugin_routes(plugin_id);
        }
    }

    /// 设置 HostApi 引用。
    pub fn set_host_api(&self, host_api: Arc<HostApi>) {
        *self.host_api.write() = Some(host_api);
    }

    /// 设置候选管道。
    pub async fn set_candidate_pipeline(&self, pipeline: CandidatePipeline) {
        *self.candidate_pipeline.write().await = pipeline;
    }

    /// 设置搜索管道。
    pub fn set_search_pipeline(&self, pipeline: SearchPipeline) {
        *self.last_top_k.write() = pipeline.top_k();
        *self.search_pipeline.write() = Some(pipeline);
    }

    /// 设置缓存的候选项。
    pub fn set_cached_candidates(&self, candidates: CachedCandidateData) {
        *self.cached_candidates.write() = candidates;
    }

    /// 设置配置管理器。
    pub fn set_config_manager(&self, config_manager: Arc<ConfigManager>) {
        *self.config_manager.write() = Some(config_manager);
    }

    /// 读取 ConfigManager 引用（未注入时为 None——CLI 场景不注入，相关逻辑直接降级）。
    fn config_manager(&self) -> Option<Arc<ConfigManager>> {
        self.config_manager.read().as_ref().cloned()
    }

    /// 注入后端翻译服务（bootstrap 注入；CLI 场景不注入，locale 降级为空串）。
    pub fn set_i18n_manager(&self, i18n: Arc<I18nManager>) {
        *self.i18n.write() = Some(i18n);
    }

    /// 当前界面语言；未注入翻译服务时返回空串（远端插件兼容空串）。
    fn current_locale(&self) -> String {
        self.i18n
            .read()
            .as_ref()
            .map(|i| i.current_language())
            .unwrap_or_default()
    }

    /// 注入会话状态推送回调（bootstrap 拿到 AppHandle 后调用；CLI 场景不注入）。
    pub fn set_session_emitter(&self, emitter: SessionStateEmitter) {
        *self.session_emitter.write() = Some(emitter);
    }

    /// 组件注册中心引用（管道重建）。
    pub fn components(&self) -> &PluginComponentRegistry {
        &self.components
    }

    /// 插件注册中心引用。
    pub fn plugin_registry(&self) -> &Arc<PluginRegistry> {
        &self.plugin_registry
    }

    // ==================== 候选缓存 ====================

    /// 获取缓存的候选项数量。
    pub fn get_cached_candidates_count(&self) -> usize {
        self.cached_candidates.read().get_candidates().len()
    }

    /// 获取所有缓存的候选项克隆。
    pub fn get_cached_candidates(&self) -> Vec<zerolaunch_plugin_api::SearchCandidate> {
        self.cached_candidates.read().get_candidates().to_vec()
    }

    /// 根据 ID 获取单个缓存的候选项。
    pub fn get_cached_candidate_by_id(
        &self,
        id: CandidateId,
    ) -> Option<zerolaunch_plugin_api::SearchCandidate> {
        self.cached_candidates.read().get_candidate(id).cloned()
    }

    /// 获取候选项的快照（计数 + 数据），单次锁获取保证一致性。
    pub fn get_candidates_snapshot(&self) -> (usize, Vec<zerolaunch_plugin_api::SearchCandidate>) {
        let guard = self.cached_candidates.read();
        let candidates = guard.get_candidates();
        (candidates.len(), candidates.to_vec())
    }

    /// 刷新候选项缓存。
    /// 所有触发源（定时/监控/手动/配置联动）共用本入口；刷新成功后记录时间戳，
    /// 供 auto-refresh 周期任务判断"距上次刷新是否已达间隔"（天然去重，避免重复刷新）。
    pub async fn refresh_candidates(&self) {
        let pipeline = self.candidate_pipeline.read().await;
        let candidates = pipeline.collect().await;
        *self.cached_candidates.write() = candidates;
        *self.last_refresh.lock() = Some(Instant::now());
    }

    /// 距最近一次刷新已过去的时长。
    /// 从未刷新过时返回 Duration::MAX（定时任务视为立即到期）。
    pub fn last_refresh_elapsed(&self) -> Duration {
        match *self.last_refresh.lock() {
            Some(t) => t.elapsed(),
            None => Duration::MAX,
        }
    }

    // ==================== 调试入口 ====================

    /// 调试用：对缓存候选项运行搜索并返回评分结果（已排序 top_k）。
    /// 参数：query - 原始查询文本（内部转为小写并折叠连续空格后匹配）。
    /// 返回：评分排序后的候选项列表；搜索管道未初始化时为空。
    pub fn debug_search(&self, query: &str) -> Vec<zerolaunch_plugin_api::ScoredCandidate> {
        let cached = self.cached_candidates.read();
        let pipeline_guard = self.search_pipeline.read();
        let Some(pipeline) = pipeline_guard.as_ref() else {
            return Vec::new();
        };
        let normalized = collapse_repeated_spaces(&query.to_lowercase());
        pipeline.search(&cached, &normalized)
    }

    /// 调试用：对缓存候选项运行全量搜索（不截断 top_k），供分数分解观察。
    /// 参数：query - 原始查询文本（内部转为小写并折叠连续空格后匹配）。
    /// 返回：完整评分排序后的候选项列表；搜索管道未初始化时为空。
    pub fn debug_search_all(&self, query: &str) -> Vec<zerolaunch_plugin_api::ScoredCandidate> {
        let cached = self.cached_candidates.read();
        let pipeline_guard = self.search_pipeline.read();
        let Some(pipeline) = pipeline_guard.as_ref() else {
            return Vec::new();
        };
        let normalized = collapse_repeated_spaces(&query.to_lowercase());
        pipeline.search_all(&cached, &normalized)
    }

    /// 调试用：对给定名称生成关键字列表（采集管道 DataSource 能力）。
    pub async fn debug_generate_keywords(&self, name: &str) -> Vec<String> {
        self.candidate_pipeline
            .read()
            .await
            .generate_keywords_for_name(name)
    }

    /// 调试用：运行索引采集并返回（耗时ms, 候选总数）。
    pub async fn debug_index_with_timing(&self) -> (u64, usize) {
        let start = std::time::Instant::now();
        self.refresh_candidates().await;
        let ms = start.elapsed().as_millis() as u64;
        (ms, self.get_cached_candidates_count())
    }

    // ==================== 会话路由 ====================

    /// 日志脱敏辅助：返回（字符长度, 截断预览），避免 INFO 日志暴露完整用户输入。
    fn log_query_preview(raw: &str) -> (usize, String) {
        const PREVIEW_LEN: usize = 24;
        let len = raw.chars().count();
        let preview: String = raw.chars().take(PREVIEW_LEN).collect();
        if len > PREVIEW_LEN {
            (len, format!("{preview}…"))
        } else {
            (len, preview)
        }
    }

    /// 返回指定通道的版本计数器：各通道独立计数，互不干扰。
    fn revision_counter(&self, channel: QueryChannel) -> &Arc<AtomicU64> {
        match channel {
            QueryChannel::Ui => &self.ui_query_revision,
            QueryChannel::Cli => &self.cli_query_revision,
            QueryChannel::Debug => &self.debug_query_revision,
        }
    }

    /// 查询过期门控：查询执行期间若有同通道更新的查询进入后端，本查询已过期。
    /// 判定单调不可逆（过期后不会再变回最新），丢弃本次结果返回空响应
    /// 优于返回过期数据（旧 SessionRouter 语义，仅记录日志，由调用方丢弃结果）。
    fn is_query_stale(&self, counter: &AtomicU64, revision: u64) -> bool {
        let latest = counter.load(Ordering::Relaxed);
        if latest != revision {
            info!(
                query_revision = revision,
                latest_query_revision = latest,
                site = "route",
                "查询过期，丢弃查询结果"
            );
        }
        latest != revision
    }

    /// 解析触发词与剩余查询内容（首词空格分隔，精确匹配触发词索引）。
    /// 语义：触发词必须带空格分隔（触发词+空格+内容），单独的触发词（无空格）
    /// 不视为命中——前端 queryStillInPanel 镜像同样要求 raw.includes(' ')。
    fn match_trigger<'a>(&self, raw_query: &'a str) -> (Option<String>, &'a str) {
        let mut parts = raw_query.splitn(2, ' ');
        let first = parts.next().unwrap_or("");
        match parts.next() {
            Some(rest) if self.trigger_index.contains_key(first) => (Some(first.to_string()), rest),
            _ => (None, raw_query),
        }
    }

    /// 路由一次查询：触发词命中 → 插件；否则 → 默认搜索。
    ///
    /// 返回 `Err(PluginError)` 表示插件匹配成功但处理失败（命中后失败不落入
    /// 默认搜索），由命令层转为 `BridgeError` 下发前端（IPC 错误通道）。
    /// 会话状态仅由 UI 通道维护：CLI/调试查询为只读辅助路径，不改写活动会话、
    /// 不推送事件（原 SessionRouter 行为保持）。
    #[tracing::instrument(skip(self, query), fields(trace_id = %trace_id, query_revision, owner))]
    pub async fn route_query(
        &self,
        trace_id: &str,
        query: &Query,
        channel: QueryChannel,
    ) -> Result<RoutedQuery, SessionDispatcherError> {
        // 从所属通道计数器分配单调递增版本号：同通道新查询取代先前查询。
        let counter = self.revision_counter(channel);
        let revision = counter.fetch_add(1, Ordering::Relaxed) + 1;
        tracing::Span::current().record("query_revision", revision);
        let (query_len, query_preview) = Self::log_query_preview(&query.raw_query);
        info!(
            query_revision = revision,
            raw_query_len = query_len,
            raw_query_preview = %query_preview,
            confirm = query.confirm,
            "查询开始"
        );

        let mut ctx = PluginContext::new(trace_id);
        ctx.with_query(query.raw_query.clone());
        ctx.set_query_revision_gate(QueryRevisionGate::new(revision, counter.clone()));
        ctx.query_channel = channel;
        ctx.locale = self.current_locale();

        // 触发词调度：首词命中索引且存在空格分隔 → 插件处理；否则 → 默认搜索。
        let (trigger, search_term) = self.match_trigger(&query.raw_query);
        if let Some(trigger) = trigger {
            let plugin_id = self
                .trigger_index
                .get(&trigger)
                .map(|e| e.value().clone())
                .unwrap_or_default();
            let plugin = self.plugin_registry.get(&plugin_id).ok_or_else(|| {
                SessionDispatcherError::InvalidState(format!(
                    "触发词索引指向的插件不存在: {}",
                    plugin_id
                ))
            })?;

            // 构造插件查询：search_term 剥离触发词（镜像原 PluginService::query 语义）。
            let plugin_query = Query {
                id: query.id.clone(),
                raw_query: query.raw_query.clone(),
                search_term: search_term.to_string(),
                confirm: query.confirm,
            };
            let mut plugin_ctx = ctx.clone();
            plugin_ctx.with_plugin_id(plugin_id.clone());
            tracing::Span::current().record("owner", plugin_id.as_str());

            match plugin.query(&plugin_ctx, &plugin_query).await {
                Ok(response) => {
                    // 提交门控：查询执行期间若有同通道更新的查询进入后端，本查询已过期
                    // （判定单调，过期后不会再变回最新），直接丢弃本次结果返回空响应。
                    if self.is_query_stale(counter, revision) {
                        return Ok(RoutedQuery {
                            response: QueryResponse::Empty,
                            generation: self.current_generation(),
                            plugin_id: Some(plugin_id),
                        });
                    }
                    // 展示形态：keep_search_bar 决定行内/全页面。
                    let presentation = match &response {
                        QueryResponse::CustomPanel {
                            keep_search_bar, ..
                        } => {
                            if *keep_search_bar {
                                PresentationMode::PluginPanel
                            } else {
                                PresentationMode::PluginImmersive
                            }
                        }
                        _ => PresentationMode::PluginPanel,
                    };
                    info!(
                        query_revision = revision,
                        target = %plugin_id,
                        "路由命中插件"
                    );
                    // 插件面板命中时无条件推送交互契约（结构保证：前端 Esc 退出不发 IPC，
                    // 后端投影滞留旧面板，同面板重入必须重推——原 panel-push 不变式）。
                    if channel == QueryChannel::Ui {
                        self.enter_session(Some(plugin_id.clone()), presentation, true);
                    }
                    Ok(RoutedQuery {
                        response,
                        generation: self.current_generation(),
                        plugin_id: Some(plugin_id),
                    })
                }
                Err(e) => {
                    // 插件匹配成功但处理失败：不静默切换默认搜索，沿 IPC 错误通道上报。
                    error!(
                        query_revision = revision,
                        target = %plugin_id,
                        error = %e,
                        "插件查询执行失败"
                    );
                    Err(SessionDispatcherError::PluginError(e.to_string()))
                }
            }
        } else {
            // 默认搜索：搜索管道 + 行内参数检测 + ListItem 映射。
            let cached = self.cached_candidates.read();
            let pipeline_guard = self.search_pipeline.read();
            let Some(pipeline) = pipeline_guard.as_ref() else {
                warn!("SearchPipeline 未初始化，返回空结果");
                return Ok(RoutedQuery {
                    response: QueryResponse::Empty,
                    generation: self.current_generation(),
                    plugin_id: None,
                });
            };
            let normalized = collapse_repeated_spaces(&query.search_term);
            let scored_candidates = pipeline.search(&cached, &normalized);

            // 提交门控（与插件分支一致）：搜索计算期间若有同通道更新的查询进入后端，
            // 本查询已过期，丢弃结果——过期返回空结果优于返回过期数据（CLI 并发/排队
            // 场景），同时避免过期查询写投影（search_state / enter_session）。
            if self.is_query_stale(counter, revision) {
                return Ok(RoutedQuery {
                    response: QueryResponse::Empty,
                    generation: self.current_generation(),
                    plugin_id: None,
                });
            }

            // 行内参数入口检测：查询以空格结尾 + 去掉空格后精确匹配某候选项的触发关键词。
            // 在 ListItem 映射之前检查，避免匹配时废弃已映射的结果。
            if query.raw_query.ends_with(' ') {
                let trimmed = query.search_term.trim();
                for candidate in &scored_candidates {
                    let Some(sc) = cached.get_candidate(candidate.candidate_id) else {
                        warn!(
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
                        if channel == QueryChannel::Ui {
                            *self.search_state.write() = SearchSubState::InlineParam {
                                candidate_id: sc.id,
                            };
                            self.enter_session(None, PresentationMode::InlineParam, false);
                        }
                        return Ok(RoutedQuery {
                            response: QueryResponse::InlineParam {
                                candidate_id: sc.id,
                                trigger_keyword: trimmed.to_string(),
                                user_arg_count,
                            },
                            generation: self.current_generation(),
                            plugin_id: None,
                        });
                    }
                }
            }

            // ListItem 映射：动作列表、占位符统计、系统参数标记、触发关键词。
            let results: Vec<ListItem> = scored_candidates
                .into_iter()
                .filter_map(|candidate| {
                    let Some(search_candidate) = cached.get_candidate(candidate.candidate_id)
                    else {
                        warn!(
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
                    Some(ListItem {
                        id: search_candidate.id,
                        title: search_candidate.name.clone(),
                        subtitle: search_candidate.target.payload().to_string(),
                        icon: search_candidate.icon.clone(),
                        score: candidate.score,
                        actions,
                        target_type: search_candidate.target.target_type().as_str().to_string(),
                        user_arg_count,
                        has_system_params,
                        trigger_keywords: search_candidate.trigger_keywords.clone(),
                    })
                })
                .collect();

            if channel == QueryChannel::Ui {
                *self.search_state.write() = SearchSubState::Search;
                self.enter_session(None, PresentationMode::Search, false);
            }
            Ok(RoutedQuery {
                response: QueryResponse::List { results },
                generation: self.current_generation(),
                plugin_id: None,
            })
        }
    }

    /// 路由一次确认：校验会话代际 → 按活动会话归属分发（插件执行 / 默认搜索执行）。
    ///
    /// 请求为 `ConfirmRequest`（命令层构造，Candidate / PluginAction 两变体统一入口）；
    /// 归属校验（插件动作须属于活动会话插件）+ 代际校验在此完成。
    /// 返回确认结局 + 会话代际——进入输入收集面板会递增代际，随响应回传前端。
    #[tracing::instrument(skip(self, req), fields(trace_id = %trace_id))]
    pub async fn route_confirm(
        &self,
        trace_id: &str,
        req: ConfirmRequest,
    ) -> Result<RoutedConfirm, SessionDispatcherError> {
        let session = self.active_session_checked(req.generation())?;
        match &session.plugin_id {
            Some(plugin_id) => {
                // 插件面板内执行：面板动作/默认动作统一经 execute_action 转发。
                let plugin = self.plugin_registry.get(plugin_id).ok_or_else(|| {
                    SessionDispatcherError::InvalidState(format!("插件不存在: {}", plugin_id))
                })?;
                let mut plugin_ctx = PluginContext::new(trace_id);
                plugin_ctx.with_plugin_id(plugin_id.clone());
                plugin_ctx.locale = self.current_locale();
                // 两条确认路径的载荷契约（统一经 bridge_confirm 通道）：
                // - PluginAction：面板动作（面板按键契约 Custom / GotoPanel）的自由 JSON，原样透传插件；
                // - Candidate：宿主确认的历史形状 {candidate_id, query_text, user_args}——
                //   第三方插件按此契约解析，行为不得破坏。
                let (action_id, payload) = match req {
                    ConfirmRequest::PluginAction {
                        plugin_id: req_plugin_id,
                        action,
                        args,
                        ..
                    } => {
                        // 归属校验：动作声明的插件必须与活动会话一致（防跨插件动作/身份错乱）。
                        if req_plugin_id != *plugin_id {
                            return Err(SessionDispatcherError::InvalidState(format!(
                                "当前会话不属于插件 {}，无法执行面板动作",
                                req_plugin_id
                            )));
                        }
                        (action, args)
                    }
                    ConfirmRequest::Candidate {
                        candidate_id,
                        action_id,
                        query_text,
                        user_args,
                        ..
                    } => (
                        action_id,
                        serde_json::json!({
                            "candidate_id": candidate_id,
                            "query_text": query_text,
                            "user_args": user_args,
                        }),
                    ),
                };
                match plugin
                    .execute_action(&plugin_ctx, &action_id, payload)
                    .await
                {
                    Ok(()) => Ok(RoutedConfirm {
                        outcome: ConfirmOutcome::Executed,
                        generation: session.generation,
                    }),
                    Err(e) => Err(SessionDispatcherError::PluginError(e.to_string())),
                }
            }
            None => {
                // 默认搜索只处理宿主候选确认；插件面板动作在插件归属分支处理。
                let ConfirmRequest::Candidate {
                    candidate_id,
                    action_id,
                    query_text,
                    user_args,
                    ..
                } = req
                else {
                    return Err(SessionDispatcherError::InvalidState(
                        "默认搜索不接受插件面板动作".to_string(),
                    ));
                };
                let state = self.search_state.read().clone();
                match state {
                    SearchSubState::InlineParam { candidate_id }
                    | SearchSubState::ParamPanel { candidate_id } => {
                        match self
                            .execute_candidate(candidate_id, &action_id, &query_text, &user_args)
                            .await
                        {
                            Ok(()) => Ok(RoutedConfirm {
                                outcome: ConfirmOutcome::Executed,
                                generation: session.generation,
                            }),
                            Err(e) => Err(SessionDispatcherError::ExecutionError(e.0)),
                        }
                    }
                    SearchSubState::Search => {
                        // 参数缺失的裁决留在后端：候选项需要参数但用户未提供 → 引导进入参数面板。
                        let user_arg_count = {
                            let cc = self.cached_candidates.read();
                            cc.get_candidate(candidate_id)
                                .map(|c| TemplateParser::count_user_args(c.target.payload()))
                                .unwrap_or(0)
                        };
                        if user_arg_count > 0 && user_args.is_empty() {
                            // 参数面板是默认搜索的子形态：子状态自持写入，投影形态自声明。
                            *self.search_state.write() =
                                SearchSubState::ParamPanel { candidate_id };
                            self.enter_session(None, PresentationMode::ParamPanel, false);
                            return Ok(RoutedConfirm {
                                outcome: ConfirmOutcome::EnterParamPanel {
                                    candidate_id,
                                    user_arg_count,
                                },
                                generation: self.current_generation(),
                            });
                        }
                        match self
                            .execute_candidate(candidate_id, &action_id, &query_text, &user_args)
                            .await
                        {
                            Ok(()) => Ok(RoutedConfirm {
                                outcome: ConfirmOutcome::Executed,
                                generation: session.generation,
                            }),
                            Err(e) => Err(SessionDispatcherError::ExecutionError(e.0)),
                        }
                    }
                }
            }
        }
    }

    /// 共享骨架：读取并克隆活动会话，校验存在（presentation 非 None）与请求代际一致。
    /// 参数：request_generation - 请求携带的代际。
    /// 返回：校验通过的活动会话快照（确认入口共用）。
    fn active_session_checked(
        &self,
        request_generation: u64,
    ) -> Result<ActiveSession, SessionDispatcherError> {
        let session = self.active_session.read().clone();
        if session.presentation == PresentationMode::None {
            return Err(SessionDispatcherError::InvalidState(
                "No active session".to_string(),
            ));
        }
        self.validate_generation(request_generation, session.generation)?;
        Ok(session)
    }

    /// 校验请求携带的代际与当前会话一致（会话归属切换后过期请求不得执行到新会话）。
    /// 参数：request_generation - 请求携带的代际；session_generation - 当前会话代际。
    /// 返回：Ok(()) 或 InvalidState 错误。
    fn validate_generation(
        &self,
        request_generation: u64,
        session_generation: u64,
    ) -> Result<(), SessionDispatcherError> {
        if request_generation != session_generation {
            return Err(SessionDispatcherError::InvalidState(format!(
                "会话已过期（期望代际 {}，实际 {}），请重试",
                session_generation, request_generation
            )));
        }
        Ok(())
    }

    /// 执行候选项：构造执行上下文 → 记录搜索行为 → 解析执行器 → 执行（含失败回退）。
    /// 参数：candidate_id - 候选项 ID；action_id - 动作 ID；query_text - 发起确认时的查询文本；
    ///       user_args - 用户参数（行内参数/参数面板场景）。
    /// 返回：Ok(()) 或执行错误。
    async fn execute_candidate(
        &self,
        candidate_id: CandidateId,
        action_id: &str,
        query_text: &str,
        user_args: &[String],
    ) -> Result<(), ConfirmError> {
        let exec_ctx = {
            let cached = self.cached_candidates.read();
            let candidate = cached
                .get_candidate(candidate_id)
                .ok_or_else(|| ConfirmError(format!("候选项未找到: id={}", candidate_id)))?;
            let snapshot = self.parameter_snapshot.lock().clone();
            let exec_ctx = ExecutionContext {
                target: candidate.target.clone(),
                display_name: candidate.name.clone(),
                user_args: user_args.to_vec(),
                parameter_snapshot: snapshot,
                locale: self.current_locale(),
            };
            if let Some(pipeline) = self.search_pipeline.read().as_ref() {
                pipeline.record(candidate_id, &cached, query_text);
            }
            exec_ctx
        };
        // 所有锁在 await 前释放；执行器解析走 ExecutorRegistry 唯一入口。
        let executor = {
            let registry = self.executor_registry.read();
            registry
                .resolve(&exec_ctx, action_id)
                .map_err(|e| ConfirmError(e.to_string()))?
        };

        match executor.execute(&exec_ctx, action_id).await {
            Ok(()) => {
                info!(
                    "[执行成功] candidate='{}' (id={}), action='{}'",
                    exec_ctx.display_name, candidate_id, action_id
                );
                Ok(())
            }
            Err(ExecutionError::ActivationFailed { fallback_action }) => {
                // 窗口唤醒失败：按配置决定是否回退执行。
                let launch_new = self
                    .config_manager()
                    .and_then(|cm| {
                        cm.get_component_setting("window-behavior-config", "launch_new_on_failure")
                    })
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                if launch_new {
                    let fallback_executor = {
                        let registry = self.executor_registry.read();
                        registry
                            .resolve_fallback(&exec_ctx, &fallback_action)
                            .map_err(|e| ConfirmError(e.to_string()))?
                    };
                    fallback_executor
                        .execute(&exec_ctx, &fallback_action)
                        .await
                        .map_err(|e| ConfirmError(e.to_string()))?;
                    info!(
                        "[执行成功] candidate='{}' (id={}), action='{}' (fallback from '{}')",
                        exec_ctx.display_name, candidate_id, fallback_action, action_id
                    );
                } else {
                    info!(
                        "[执行忽略] candidate='{}' (id={}), activation failed for '{}', fallback disabled",
                        exec_ctx.display_name, candidate_id, action_id
                    );
                }
                Ok(())
            }
            Err(e) => Err(ConfirmError(e.to_string())),
        }
    }

    // ==================== 会话维护 ====================

    /// 进入会话投影：归属/形态变化时递增代际并更新活动会话；
    /// 投影变化或插件面板路由命中时推送会话事件（幂等，前端无条件接受）。
    ///
    /// `always_push`：插件面板命中时恒为 true——前端 Esc 退出经 doQuery('') 短路
    /// （不发 IPC），后端投影无从感知退出；同面板重入时若仅按「投影变化」推送，
    /// 交互契约将永久丢失（原 panel-push-unconditional 不变式，结构保证）。
    /// 进入会话投影：归属/形态变化时递增代际并更新活动会话；
    /// 投影变化或插件面板路由命中时推送会话事件（幂等，前端无条件接受）。
    ///
    /// `plugin_id`：None = 宿主默认搜索（含行内参数/参数面板子状态）；Some(id) = 插件。
    /// `always_push`：插件面板命中时恒为 true——前端 Esc 退出经 doQuery('') 短路
    /// （不发 IPC），后端投影无从感知退出；同面板重入时若仅按「投影变化」推送，
    /// 交互契约将永久丢失（原 panel-push-unconditional 不变式，结构保证）。
    fn enter_session(
        &self,
        plugin_id: Option<String>,
        presentation: PresentationMode,
        always_push: bool,
    ) {
        self.enter_session_inner(plugin_id, presentation, always_push, None);
    }

    /// 进入会话投影的内部实现：`content` 为热键唤醒携带的面板渲染载荷，
    /// 仅唤醒路径非 None（关键词查询路径的载荷随 bridge_query 响应下发）。
    fn enter_session_inner(
        &self,
        plugin_id: Option<String>,
        presentation: PresentationMode,
        always_push: bool,
        content: Option<PluginPanelContent>,
    ) {
        let mut session = self.active_session.write();
        let changed = session.plugin_id != plugin_id || session.presentation != presentation;
        if !changed && !always_push {
            return;
        }
        // 代际随会话投影写入递增（单一数据源：ActiveSession.generation）。
        let generation = if changed {
            session.generation + 1
        } else {
            session.generation
        };
        if changed {
            *session = ActiveSession {
                generation,
                plugin_id: plugin_id.clone(),
                presentation,
            };
        }
        drop(session);
        self.push_session_state(generation, &plugin_id, presentation, content);
    }

    /// 推送会话状态事件（无 emitter 的 CLI 场景直接跳过）。
    fn push_session_state(
        &self,
        generation: u64,
        plugin_id: &Option<String>,
        presentation: PresentationMode,
        content: Option<PluginPanelContent>,
    ) {
        let Some(emitter) = self.session_emitter.read().clone() else {
            return;
        };
        let (panel, interaction, trigger_keywords) = match plugin_id {
            Some(id) => {
                let plugin = self.plugin_registry.get(id);
                (
                    Some(PluginPanelInfo {
                        plugin_id: id.clone(),
                        panel_id: "main".to_string(),
                    }),
                    plugin.as_ref().map(|p| p.interaction_policy()),
                    plugin
                        .as_ref()
                        .map(|p| p.metadata().trigger_keywords.clone())
                        .unwrap_or_default(),
                )
            }
            None => (None, None, Vec::new()),
        };
        emitter(SessionStateEvent {
            generation,
            presentation,
            panel,
            interaction,
            trigger_keywords,
            panel_content: content,
        });
    }

    /// 会话重置：参数面板/行内参数/搜索恒重置；插件模式仅当 `reset_plugins` 为 true 时重置
    /// （支持隐藏/显示间保持插件面板状态）。返回 true 表示实际执行了重置。
    pub fn reset_session(&self, reset_plugins: bool) -> bool {
        let mut session = self.active_session.write();
        let should_reset = match &session.plugin_id {
            Some(_) => reset_plugins,
            None => session.presentation != PresentationMode::None,
        };
        if !should_reset {
            return false;
        }
        let changed = session.presentation != PresentationMode::None;
        let generation = if changed {
            session.generation + 1
        } else {
            session.generation
        };
        if changed {
            *session = ActiveSession {
                generation,
                plugin_id: None,
                presentation: PresentationMode::None,
            };
        }
        // 默认搜索子状态重置（InlineParam/ParamPanel 属本调度器内嵌状态；
        // 插件面板状态由插件自己管理，宿主不感知）。
        *self.search_state.write() = SearchSubState::Search;
        *self.parameter_snapshot.lock() = ParameterSnapshot::empty();
        drop(session);
        // 会话结束投影：唯一事件通道推送（原 session-reset 事件已删除）。
        if changed {
            self.push_session_state(generation, &None, PresentationMode::None, None);
        }
        true
    }

    /// 当前活动会话（克隆）。
    pub fn current_session(&self) -> ActiveSession {
        self.active_session.read().clone()
    }

    /// 当前展示形态（CLI /v1/session 等只读场景）。
    pub fn current_presentation(&self) -> PresentationMode {
        self.active_session.read().presentation
    }

    /// 当前会话代际。
    pub fn current_generation(&self) -> u64 {
        self.active_session.read().generation
    }

    /// 重新推送当前会话投影（配置变更后调用，面板内调整防抖等即时生效）。
    pub fn reemit_current_session(&self) {
        let session = self.active_session.read().clone();
        if session.presentation == PresentationMode::None {
            return;
        }
        self.push_session_state(
            session.generation,
            &session.plugin_id,
            session.presentation,
            None,
        );
    }

    /// 搜索栏唤醒：捕获系统参数快照。
    pub async fn on_search_bar_wake(&self) -> Result<(), SessionDispatcherError> {
        let host_api = self.host_api.read().clone().ok_or_else(|| {
            SessionDispatcherError::NotInitialized(
                "HostApi not initialized in SessionDispatcher".to_string(),
            )
        })?;
        let snapshot = host_api.capture_parameter_snapshot().await;
        *self.parameter_snapshot.lock() = snapshot;
        debug!("📸 搜索栏唤醒，系统参数快照已捕获");
        Ok(())
    }

    /// 热键唤醒插件（完全插件模式）：捕获参数快照 → 空查询 → 进入全页面接管会话。
    /// 响应必须为 CustomPanel 且 keep_search_bar=false（全页面接管契约，断言强制）；
    /// 载荷经会话事件 panelContent 一并推送（窗口隐藏时前端无查询响应可依赖）。
    /// 非 CustomPanel 响应（List/Empty）属契约违约，返回错误（前端无载荷可渲染，
    /// 静默进入会导致前后端投影失步）。
    pub async fn wake_plugin(&self, plugin_id: &str) -> Result<(), SessionDispatcherError> {
        // 启用校验：禁用插件不可被热键唤醒（前端热键表可能残留过期条目，
        // 后端为权威裁决，与触发词路由的「禁用即不路由」语义一致）。
        if !self.is_plugin_enabled(plugin_id) {
            return Err(SessionDispatcherError::InvalidState(format!(
                "热键唤醒的插件未启用: {}",
                plugin_id
            )));
        }
        let host_api: Arc<HostApi> = self.host_api.read().clone().ok_or_else(|| {
            SessionDispatcherError::NotInitialized(
                "HostApi not initialized in SessionDispatcher".to_string(),
            )
        })?;
        let snapshot = host_api.capture_parameter_snapshot().await;
        *self.parameter_snapshot.lock() = snapshot;

        let plugin = self.plugin_registry.get(plugin_id).ok_or_else(|| {
            SessionDispatcherError::InvalidState(format!("热键唤醒的插件不存在: {}", plugin_id))
        })?;

        let trace_id = crate::utils::trace_id::generate_trace_id();
        let mut ctx = PluginContext::new(&trace_id);
        ctx.with_query(trace_id.clone());
        ctx.with_plugin_id(plugin_id.to_string());
        ctx.locale = self.current_locale();
        let query = Query {
            id: trace_id,
            raw_query: String::new(),
            search_term: String::new(),
            confirm: false,
        };

        let response = plugin.query(&ctx, &query).await.map_err(|e| {
            error!(
                target = plugin_id,
                error = %e,
                "热键唤醒插件查询失败"
            );
            SessionDispatcherError::PluginError(e.to_string())
        })?;

        // 展示形态与载荷：热键唤醒 = 完全插件模式 = 全页面接管（PluginImmersive）。
        // keep_search_bar: true（行内面板）在热键唤醒语义下是不存在路径——声明 hotkey
        // 的插件必须设计为全面板。出现即契约被破坏（插件声明热键却返回行内形态），
        // 立即 panic 暴露以便定位，不做静默降级（断言为宿主不变量，插件违约即宿主逻辑缺陷）。
        // 非 CustomPanel 响应（List/Empty）属契约违约，返回错误（前端无载荷可渲染，
        // 静默进入会导致前后端投影失步）。
        let (presentation, content) = match response {
            QueryResponse::CustomPanel {
                panel_type,
                data,
                actions,
                keep_search_bar,
            } => {
                assert!(
                    !keep_search_bar,
                    "热键唤醒插件 {} 返回 keep_search_bar=true（行内面板）——热键唤醒仅支持全页面接管（PluginImmersive），插件契约违约",
                    plugin_id
                );
                (
                    PresentationMode::PluginImmersive,
                    Some(PluginPanelContent {
                        panel_type,
                        data,
                        actions: actions.into_iter().map(PanelContentAction::from).collect(),
                    }),
                )
            }
            _ => {
                warn!(
                    target = plugin_id,
                    "热键唤醒插件未返回 CustomPanel 面板响应"
                );
                return Err(SessionDispatcherError::PluginError(format!(
                    "热键唤醒的插件 {} 未返回 CustomPanel 面板响应",
                    plugin_id
                )));
            }
        };
        info!(
            target = plugin_id,
            presentation = presentation.as_str(),
            "热键唤醒插件"
        );
        self.enter_session_inner(Some(plugin_id.to_string()), presentation, true, content);
        Ok(())
    }

    // ==================== 管道与配置事件 ====================

    /// 重建候选管道：从 ConfigManager 构建 → 注入偏置规则 → 替换管道 → 刷新候选项。
    async fn rebuild_candidate_pipeline(&self) {
        let Some(cm) = self.config_manager() else {
            return;
        };
        let mut new_pipeline = self.components.build_candidate_pipeline(&cm);
        // 从 BiasConfig 注入固定偏移量规则
        let rules = cm
            .get_settings("bias-config")
            .and_then(|v| serde_json::from_value::<BiasSettings>(v).ok())
            .map(|settings| bias_settings_to_rules(&settings))
            .unwrap_or_default();
        new_pipeline.set_bias_rules(rules);
        *self.candidate_pipeline.write().await = new_pipeline;
        self.refresh_candidates().await;
    }

    /// 根据当前注册的搜索引擎和分数增强器重建搜索管道。
    pub fn rebuild_search_pipeline(&self) {
        let Some(cm) = self.config_manager() else {
            return;
        };
        let top_k = *self.last_top_k.read();
        match self.components.build_search_pipeline(&cm, top_k) {
            Some(pipeline) => {
                info!("搜索管道已重建 (top_k: {})", pipeline.top_k());
                *self.search_pipeline.write() = Some(pipeline);
            }
            None => {
                warn!("没有启用的搜索引擎，无法重建搜索管道");
            }
        }
    }

    /// 处理配置变更事件。
    pub async fn handle_config_event(&self, event: &ConfigEvent) {
        match event {
            ConfigEvent::SettingsChanged {
                component_type,
                component_id,
            } => {
                debug!("配置变更事件: {} ({:?})", component_id, component_type);
                match component_type {
                    ComponentType::DataSource
                    | ComponentType::KeywordOptimizer
                    | ComponentType::KeywordInjector => {
                        info!("数据源/关键词优化器配置变更，刷新候选项缓存");
                        self.refresh_candidates().await;
                    }
                    ComponentType::SearchEngine | ComponentType::ScoreBooster => {
                        info!("搜索引擎/分数增强器配置变更，重建搜索管道");
                        self.rebuild_search_pipeline();
                    }
                    ComponentType::Core => {
                        debug!("Core 组件({})配置变更，无需响应", component_id);
                    }
                    ComponentType::BiasRule => {
                        info!("偏置规则配置变更，重建候选管道");
                        self.rebuild_candidate_pipeline().await;
                    }
                    _ => {
                        debug!("{:?} 配置变更，无需响应", component_type);
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
                    ComponentType::DataSource
                    | ComponentType::KeywordOptimizer
                    | ComponentType::KeywordInjector
                    | ComponentType::BiasRule => {
                        info!("组件或偏置规则启用状态变更，重建候选管道");
                        self.rebuild_candidate_pipeline().await;
                    }
                    ComponentType::SearchEngine | ComponentType::ScoreBooster => {
                        info!("搜索引擎/分数增强器启用状态变更，重建搜索管道");
                        self.rebuild_search_pipeline();
                    }
                    ComponentType::Plugin => {
                        info!(
                            "插件启用状态变更，更新触发词索引: {} enabled={}",
                            component_id, enabled
                        );
                        // 内置插件 component_id 即 plugin_id；第三方插件组件 id 等于 plugin_id 时同样命中，
                        // 不相等时由 plugin_set_enabled 命令按 plugin_id 直调兜底。
                        self.set_plugin_enabled(component_id, *enabled);
                    }
                    ComponentType::ActionExecutor | ComponentType::Core => {
                        debug!("ActionExecutor/Core 启用状态变更，无需响应");
                    }
                }
            }
            ConfigEvent::Registered { .. } | ConfigEvent::Unregistered { .. } => {}
            ConfigEvent::PluginRegistered(adapters) => {
                info!("第三方插件运行时组件已注册: {}", adapters.plugin_id);
                for comp in &adapters.components {
                    if let Some(ds) = comp.clone().as_data_source() {
                        self.components.register_data_source(ds);
                    }
                    if let Some(ex) = comp.clone().as_action_executor() {
                        self.register_executor(ex);
                    }
                    if let Some(p) = comp.clone().as_plugin() {
                        // 按组件持久化启用状态决定是否建立触发词路由（禁用插件重启后不路由）
                        let enabled = self
                            .config_manager()
                            .map(|cm| cm.is_enabled(comp.core.component_id()))
                            .unwrap_or(true);
                        self.register_plugin_with_triggers(p.clone(), enabled);
                        // 远端插件 init（内置 init 在 bootstrap Phase B 统一执行）：
                        // 通知插件进程完成初始化（无宿主句柄，平台能力经 host RPC）。
                        // fire-and-forget：init 不阻塞配置事件循环（插件挂起时
                        // RPC 超时可能达 10s，串行循环内会拖累后续配置事件）；
                        // 失败仅记 error——注册已完成，进程存活时查询等仍可用。
                        let mut init_ctx = PluginContext::new("init");
                        init_ctx.locale = self.current_locale();
                        let plugin_id = adapters.plugin_id.clone();
                        let component_id = comp.core.component_id().to_string();
                        tauri::async_runtime::spawn(async move {
                            if let Err(e) = p.init(&init_ctx, None).await {
                                error!(
                                    "远端插件 {} 组件 {} init 失败: {}",
                                    plugin_id, component_id, e
                                );
                            }
                        });
                    }
                }
                // 重建候选管道以包含新组件
                self.rebuild_candidate_pipeline().await;
            }
            ConfigEvent::PluginUnregistered(adapters) => {
                info!("第三方插件运行时组件已解注册: {}", adapters.plugin_id);
                self.unregister_plugin(&adapters.plugin_id);
                for comp in &adapters.components {
                    if comp.is_data_source() {
                        self.components
                            .unregister_data_source(comp.core.component_id());
                    }
                    if comp.is_action_executor() {
                        self.unregister_executor(comp.core.component_id());
                    }
                }
                // 重建候选管道以移除已解注册的组件
                self.rebuild_candidate_pipeline().await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::collections::HashSet;
    use zerolaunch_plugin_api::config::{
        ComponentCore, ComponentType, Configurable, SettingDefinition,
    };
    use zerolaunch_plugin_api::mock::*;
    use zerolaunch_plugin_api::services::resource::AppResourceService;
    use zerolaunch_plugin_api::services::storage::storage_service::StorageService;
    use zerolaunch_plugin_api::services::timer::TokioTimerManager;
    use zerolaunch_plugin_api::{
        PlatformCapabilities, PluginError, PluginHandle, PluginKind, PluginMetadata,
    };

    /// 构建仅含桩组件的 HostApi（测试专用，不触达真实平台能力）。
    /// 镜像 builtin_registry 测试的组件清单。
    fn test_host_api() -> Arc<HostApi> {
        let storage: Arc<dyn StorageService> = Arc::new(StubStorageService);
        let api = HostApi::builder("mock_icons".to_string())
            .capabilities(PlatformCapabilities::new(HashSet::new()))
            .icon_extractor(Arc::new(StubIconExtractor))
            .shell_executor(Arc::new(StubShellExecutor::default()))
            .window_manager(Arc::new(StubWindowManager))
            .path_resolver(Arc::new(StubPathResolver))
            .app_enumerator(Arc::new(StubAppEnumerator))
            .app_launcher(Arc::new(StubAppLauncher))
            .lnk_resolver(Arc::new(StubLnkResolver))
            .resource_loader(Arc::new(StubResourceLoader))
            .parameter_resolver(Arc::new(StubParameterResolver))
            .parameter_providers(
                Arc::new(StubSystemParameterProvider),
                Arc::new(StubSystemParameterProvider),
                Arc::new(StubSystemParameterProvider),
            )
            .autostart_manager(Arc::new(StubAutoStartManager))
            .hotkey_manager(Arc::new(StubHotkeyManager))
            .installation_monitor(Arc::new(StubInstallationMonitor))
            .timer_manager(Arc::new(TokioTimerManager::new()))
            .storage_service(storage)
            .app_resource(Arc::new(AppResourceService::new("mock_icons".to_string())))
            .focus_monitor(Arc::new(StubFocusMonitor))
            .clipboard_manager(Arc::new(StubClipboardManager))
            .notify_callback(|_, _| {})
            .hide_window_callback(|| {})
            .show_window_callback(|| {})
            .is_window_visible_callback(|| false)
            .window_positioner(Arc::new(StubWindowPositioner))
            .set_window_position_callback(|_, _| {})
            .build()
            .expect("构建测试 HostApi 失败");
        Arc::new(api)
    }

    /// 触发词路由测试用最小插件桩 —— 仅填充元数据（触发词），其余方法空实现。
    /// 避免测试模块引用内置实现（plugin_framework 层不得依赖 builtin_plugin，P3 层级）。
    struct TriggerStubPlugin {
        metadata: PluginMetadata,
        core: ComponentCore,
    }

    impl TriggerStubPlugin {
        fn with_trigger(trigger: &str) -> Self {
            Self::with_trigger_and_id(trigger, &format!("test.{}", trigger))
        }

        /// 指定插件 id 的构造器：允许两个插件声明相同触发词（用于冲突路径测试）。
        fn with_trigger_and_id(trigger: &str, id: &str) -> Self {
            Self {
                metadata: PluginMetadata {
                    id: id.to_string(),
                    name: format!("stub-{}", trigger),
                    version: "0.1.0".to_string(),
                    description: "测试桩".to_string(),
                    author: "test".to_string(),
                    trigger_keywords: vec![trigger.to_string()],
                    supported_os: Vec::new(),
                    priority: 0,
                    kind: PluginKind::Builtin,
                    hotkey: None,
                },
                core: ComponentCore::new(
                    id.to_string(),
                    "测试桩".to_string(),
                    "触发词路由测试".to_string(),
                    ComponentType::Plugin,
                    0,
                ),
            }
        }
    }

    impl Configurable for TriggerStubPlugin {
        fn core(&self) -> &ComponentCore {
            &self.core
        }

        fn setting_schema(&self) -> Vec<SettingDefinition> {
            Vec::new()
        }
    }

    #[async_trait]
    impl Plugin for TriggerStubPlugin {
        fn metadata(&self) -> &PluginMetadata {
            &self.metadata
        }

        async fn init(
            &self,
            _ctx: &PluginContext,
            _handle: Option<Arc<PluginHandle>>,
        ) -> Result<(), PluginError> {
            Ok(())
        }

        async fn query(
            &self,
            _ctx: &PluginContext,
            _query: &Query,
        ) -> Result<QueryResponse, PluginError> {
            Ok(QueryResponse::Empty)
        }

        async fn execute_action(
            &self,
            _ctx: &PluginContext,
            _action_id: &str,
            _payload: serde_json::Value,
        ) -> Result<(), PluginError> {
            Ok(())
        }
    }

    /// 注册带触发词的插件后，match_trigger 必须命中。
    /// 回归：此前 bootstrap 只调 plugin_registry().register（不写触发词索引），
    /// 导致内置触发式插件（translator/calculator）路由恒 miss、静默落入默认搜索。
    #[test]
    fn register_plugin_with_triggers_enables_match_trigger() {
        let dispatcher = SessionDispatcher::new(Arc::new(PluginRegistry::new()));
        let plugin: Arc<dyn Plugin> = Arc::new(TriggerStubPlugin::with_trigger("="));
        dispatcher.register_plugin_with_triggers(plugin, true);

        // 触发词 + 空格分隔 → 命中并切出搜索词
        assert_eq!(
            dispatcher.match_trigger("= 1+1"),
            (Some("=".to_string()), "1+1")
        );
        // 无空格分隔 → 不命中（与前端 queryStillInPanel 镜像判定一致）
        assert_eq!(dispatcher.match_trigger("=1+1"), (None, "=1+1"));
    }

    /// 禁用插件后触发词不再命中；重新启用后恢复。
    /// 回归：config_set_enabled 对 Plugin 类型组件曾「无需响应」，禁用后插件仍可路由使用。
    #[test]
    fn set_plugin_enabled_toggles_trigger_index() {
        let dispatcher = SessionDispatcher::new(Arc::new(PluginRegistry::new()));
        let plugin: Arc<dyn Plugin> = Arc::new(TriggerStubPlugin::with_trigger("="));
        let plugin_id = plugin.metadata().id.clone();
        dispatcher.register_plugin_with_triggers(plugin, true);

        // 注册后命中
        assert_eq!(
            dispatcher.match_trigger("= 1+1"),
            (Some("=".to_string()), "1+1")
        );

        // 禁用 → 触发词移除，不再路由到该插件
        dispatcher.set_plugin_enabled(&plugin_id, false);
        assert_eq!(dispatcher.match_trigger("= 1+1"), (None, "= 1+1"));

        // 启用 → 触发词恢复
        dispatcher.set_plugin_enabled(&plugin_id, true);
        assert_eq!(
            dispatcher.match_trigger("= 1+1"),
            (Some("=".to_string()), "1+1")
        );

        // 对未注册插件启用：无害（无触发词可恢复）
        dispatcher.set_plugin_enabled("not-registered", true);
        assert_eq!(
            dispatcher.match_trigger("= 1+1"),
            (Some("=".to_string()), "1+1")
        );
    }

    /// 持久化为禁用的插件注册时不建立触发词路由（重启后保持禁用语义）。
    #[test]
    fn register_disabled_plugin_skips_trigger_index() {
        let dispatcher = SessionDispatcher::new(Arc::new(PluginRegistry::new()));
        let plugin: Arc<dyn Plugin> = Arc::new(TriggerStubPlugin::with_trigger("="));
        let plugin_id = plugin.metadata().id.clone();
        dispatcher.register_plugin_with_triggers(plugin, false);

        // 注册了但触发词未写入：不路由
        assert_eq!(dispatcher.match_trigger("= 1+1"), (None, "= 1+1"));

        // 启用后恢复路由
        dispatcher.set_plugin_enabled(&plugin_id, true);
        assert_eq!(
            dispatcher.match_trigger("= 1+1"),
            (Some("=".to_string()), "1+1")
        );
    }

    /// 启用恢复触发词时遇到被其他插件占用的词：跳过并保留既有绑定（不覆盖）。
    #[test]
    fn enable_recovery_skips_conflicting_keyword() {
        let dispatcher = SessionDispatcher::new(Arc::new(PluginRegistry::new()));
        // 插件 A 与 B 声明相同触发词但 id 不同
        let plugin_a: Arc<dyn Plugin> =
            Arc::new(TriggerStubPlugin::with_trigger_and_id("=", "plugin-a"));
        let plugin_b: Arc<dyn Plugin> =
            Arc::new(TriggerStubPlugin::with_trigger_and_id("=", "plugin-b"));
        // A 注册并占用 "="
        dispatcher.register_plugin_with_triggers(plugin_a, true);
        assert_eq!(
            dispatcher.trigger_index.get("=").map(|r| r.clone()),
            Some("plugin-a".to_string())
        );

        // A 禁用（释放 "="）→ B 注册（无冲突，占用 "="）
        dispatcher.set_plugin_enabled("plugin-a", false);
        dispatcher.register_plugin_with_triggers(plugin_b, true);
        assert_eq!(
            dispatcher.trigger_index.get("=").map(|r| r.clone()),
            Some("plugin-b".to_string())
        );

        // A 重新启用："=" 已被 B 占用 → 跳过恢复，B 绑定不被覆盖
        dispatcher.set_plugin_enabled("plugin-a", true);
        assert_eq!(
            dispatcher.match_trigger("= 1+1"),
            (Some("=".to_string()), "1+1")
        );
        assert_eq!(
            dispatcher.trigger_index.get("=").map(|r| r.clone()),
            Some("plugin-b".to_string())
        );
    }

    /// 热键唤醒测试用面板桩 —— query 返回 CustomPanel（形态由构造参数决定）。
    struct PanelStubPlugin {
        metadata: PluginMetadata,
        core: ComponentCore,
        keep_search_bar: bool,
    }

    impl PanelStubPlugin {
        fn new() -> Self {
            Self::with_keep_search_bar(false)
        }

        /// 指定 keep_search_bar 的构造器：false = 全页面接管（合法）；true = 行内面板（契约违约路径测试）。
        fn with_keep_search_bar(keep_search_bar: bool) -> Self {
            Self {
                metadata: PluginMetadata {
                    id: "test.panel".to_string(),
                    name: "面板桩".to_string(),
                    version: "0.1.0".to_string(),
                    description: "热键唤醒测试".to_string(),
                    author: "test".to_string(),
                    trigger_keywords: Vec::new(),
                    supported_os: Vec::new(),
                    priority: 0,
                    kind: PluginKind::Builtin,
                    hotkey: Some("Ctrl+E".to_string()),
                },
                core: ComponentCore::new(
                    "test.panel".to_string(),
                    "面板桩".to_string(),
                    "热键唤醒测试".to_string(),
                    ComponentType::Plugin,
                    0,
                ),
                keep_search_bar,
            }
        }
    }

    impl Configurable for PanelStubPlugin {
        fn core(&self) -> &ComponentCore {
            &self.core
        }

        fn setting_schema(&self) -> Vec<SettingDefinition> {
            Vec::new()
        }
    }

    #[async_trait]
    impl Plugin for PanelStubPlugin {
        fn metadata(&self) -> &PluginMetadata {
            &self.metadata
        }

        async fn init(
            &self,
            _ctx: &PluginContext,
            _handle: Option<Arc<PluginHandle>>,
        ) -> Result<(), PluginError> {
            Ok(())
        }

        async fn query(
            &self,
            _ctx: &PluginContext,
            _query: &Query,
        ) -> Result<QueryResponse, PluginError> {
            Ok(QueryResponse::CustomPanel {
                panel_type: "test-panel".to_string(),
                data: serde_json::json!({ "hello": "world" }),
                actions: Vec::new(),
                keep_search_bar: self.keep_search_bar,
            })
        }

        async fn execute_action(
            &self,
            _ctx: &PluginContext,
            _action_id: &str,
            _payload: serde_json::Value,
        ) -> Result<(), PluginError> {
            Ok(())
        }
    }

    /// 热键唤醒：空查询 → 插件 CustomPanel → 进入全页面会话并推送含载荷的会话事件。
    #[tokio::test]
    async fn wake_plugin_enters_immersive_session_with_content() {
        let dispatcher = SessionDispatcher::new(Arc::new(PluginRegistry::new()));
        dispatcher.set_host_api(test_host_api());
        let plugin: Arc<dyn Plugin> = Arc::new(PanelStubPlugin::new());
        dispatcher.register_plugin_with_triggers(plugin, true);

        // 捕获会话事件（后端权威投影推送的唯一通道）
        let events = Arc::new(Mutex::new(Vec::new()));
        let capture = events.clone();
        dispatcher.set_session_emitter(Arc::new(move |event| {
            capture.lock().push(event);
        }));

        dispatcher
            .wake_plugin("test.panel")
            .await
            .expect("热键唤醒应成功");

        let session = dispatcher.current_session();
        assert_eq!(session.plugin_id.as_deref(), Some("test.panel"));
        assert_eq!(session.presentation, PresentationMode::PluginImmersive);

        let events = events.lock();
        let event = events.last().expect("应推送会话事件");
        assert_eq!(
            event.panel.as_ref().map(|p| p.plugin_id.as_str()),
            Some("test.panel")
        );
        let content = event
            .panel_content
            .as_ref()
            .expect("唤醒推送应携带面板载荷");
        assert_eq!(content.panel_type, "test-panel");
        assert_eq!(content.data, serde_json::json!({ "hello": "world" }));
    }

    /// 禁用插件不可被热键唤醒（后端权威校验，前端热键表残留过期条目时兜底）。
    #[tokio::test]
    async fn wake_plugin_rejects_disabled_plugin() {
        let dispatcher = SessionDispatcher::new(Arc::new(PluginRegistry::new()));
        dispatcher.set_host_api(test_host_api());
        let plugin: Arc<dyn Plugin> = Arc::new(PanelStubPlugin::new());
        // 禁用状态注册（enabled=false）→ 不在启用集合
        dispatcher.register_plugin_with_triggers(plugin, false);

        let err = dispatcher.wake_plugin("test.panel").await.unwrap_err();
        assert!(
            err.to_string().contains("未启用"),
            "应拒绝唤醒禁用插件: {}",
            err
        );
    }

    /// 热键唤醒要求 CustomPanel 契约：List/Empty 响应属违约，报错避免前后端投影失步。
    #[tokio::test]
    async fn wake_plugin_rejects_non_custom_panel_response() {
        let dispatcher = SessionDispatcher::new(Arc::new(PluginRegistry::new()));
        dispatcher.set_host_api(test_host_api());
        // TriggerStubPlugin 的 query 返回 Empty（非 CustomPanel）
        let plugin: Arc<dyn Plugin> = Arc::new(TriggerStubPlugin::with_trigger("="));
        let plugin_id = plugin.metadata().id.clone();
        dispatcher.register_plugin_with_triggers(plugin, true);

        let err = dispatcher.wake_plugin(&plugin_id).await.unwrap_err();
        assert!(
            err.to_string().contains("CustomPanel"),
            "应拒绝非 CustomPanel 响应: {}",
            err
        );
    }

    /// 热键唤醒 = 全页面接管：keep_search_bar=true（行内面板）是不存在路径，
    /// 断言强制暴露（契约违约即宿主逻辑缺陷，不做静默降级）。
    #[tokio::test]
    #[should_panic(expected = "keep_search_bar=true")]
    async fn wake_plugin_panics_on_keep_search_bar() {
        let dispatcher = SessionDispatcher::new(Arc::new(PluginRegistry::new()));
        dispatcher.set_host_api(test_host_api());
        let plugin: Arc<dyn Plugin> = Arc::new(PanelStubPlugin::with_keep_search_bar(true));
        dispatcher.register_plugin_with_triggers(plugin, true);

        // 应 panic（断言消息含 keep_search_bar=true），不返回
        let _ = dispatcher.wake_plugin("test.panel").await;
    }
}
