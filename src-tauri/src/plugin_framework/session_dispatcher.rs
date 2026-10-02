//! SessionDispatcher —— 会话调度器（取代 SessionRouter）。
//!
//! 直接内嵌默认搜索与触发式插件的查询/确认逻辑（SessionRouter 式，无流程抽象层）：
//! 触发词路由、默认搜索（行内参数检测、ListItem 构造、参数面板引导）、候选项执行
//! （含 ActivationFailed fallback）、插件 execute_action 转发。
//! 会话系统层保留：代际、session-state 事件、面板动作通道、管道重建。
//!
//! 查询入口按能力拆分：`evaluate_query` 为无副作用的求值，仅 `route_query_ui`
//! 具备会话写入能力；CLI（`route_query_cli`）与插件面板（`route_query_panel`）为
//! 只读辅助路径，结构上不具备改写活动会话、推送事件的能力。

use dashmap::DashSet;
use parking_lot::{Mutex, RwLock};
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};
use zerolaunch_plugin_api::config::ComponentType;
use zerolaunch_plugin_api::services::icon_request::IconRequest;
use zerolaunch_plugin_api::services::parameter::template_parser::{Placeholder, TemplateParser};
use zerolaunch_plugin_api::services::ParameterSnapshot;
use zerolaunch_plugin_api::{
    keyword_trigger_match, CachedCandidateData, CandidateId, ExecutionContext, ExecutionError,
    ExecutionTarget, ListItem, PanelInteraction, Plugin, PluginContext, PluginKind, PluginMetadata,
    PluginMode, Query, QueryChannel, QueryResponse, QueryRevisionGate, ScoredCandidate,
    SearchCandidate,
};

use super::candidate_pipeline::CandidatePipeline;
use super::component_registry::PluginComponentRegistry;
use super::executor_registry::ExecutorRegistry;
use super::registry::PluginRegistry;
use super::search_pipeline::SearchPipeline;
use super::session_state::{
    ActiveSession, HostView, InputMatch, PluginIdentity, PluginPanelContent, PluginView,
    ResultActionDto, SessionOwner, SessionStateEmitter, SessionStateEvent,
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

/// 查询求值结果（无会话副作用）——由 `evaluate_query` 产出，供三个查询入口消费。
#[derive(Debug)]
struct EvaluatedQuery {
    /// 展示响应（`current == false` 时恒为空响应）。
    response: QueryResponse,
    /// 实际处理本次查询的会话归属（None = 宿主默认搜索）。
    owner: Option<String>,
    /// 本次求值是否为「最新且有效」的结果。false 的两个来源语义一致（均不写会话投影）：
    /// ① 已被同通道更新的查询取代（过期丢弃）；② 求值前置条件未就绪（搜索管道未初始化）。
    current: bool,
    /// 命中的插件路由来源（None = 默认搜索或面板直调）：
    /// 决定会话事件下发给前端的输入匹配模型。
    match_model: Option<InputMatch>,
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
        /// 前端最后一次观测到的候选缓存世代（refresh_candidates 递增）。
        /// 后端校验与当前缓存一致，防止刷新后 id 漂移执行错误候选。
        candidate_generation: u64,
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

/// 路由裁决结果：接管的插件与其派生查询词。
///
/// 使用范围：仅 `SessionDispatcher` 路由内部——`locate_plugin` 产出、`evaluate_query` 消费；
/// 不参与序列化，不进入插件协议。
struct LocatedPlugin {
    /// 命中的插件 id（注册表键）。
    plugin_id: String,
    /// 传给插件的查询词：框架默认关键词判定为剥离触发词后的原文；插件自定义匹配为原始输入。
    search_term: String,
    /// 命中来源（下发前端作为输入匹配模型）；面板直调等无路由裁决的场景为 None。
    match_model: Option<InputMatch>,
}

/// 路由阶段询问全部自定义匹配插件的整体截止时间。
///
/// 使用范围：仅 `SessionDispatcher::locate_plugin`。单次远程匹配另有 100ms RPC 超时；
/// 此处是"全部插件收齐"的总兜底——超时后未返回的插件按不接管处理并告警，绝不阻塞输入。
const ROUTE_DEADLINE: Duration = Duration::from_millis(150);

pub struct SessionDispatcher {
    /// 插件注册中心（插件 init 在 bootstrap 完成）。
    plugin_registry: Arc<PluginRegistry>,
    /// 插件级启用状态集合（注册/启停时同步；路由与 wake_plugin 启用校验的权威依据——
    /// 禁用插件即使前端热键表残留也不得被唤醒）。DashSet 并发安全，免去外部锁。
    enabled_plugins: DashSet<String>,
    /// 活动会话（权威投影，代际随其写入递增）。
    active_session: RwLock<ActiveSession>,

    // ---- 横切：默认搜索服务（Dispatcher 直接持有，管道重建对查询无感）----
    search_pipeline: Arc<RwLock<Option<SearchPipeline>>>,
    candidate_pipeline: Arc<tokio::sync::RwLock<CandidatePipeline>>,
    cached_candidates: Arc<RwLock<Arc<CachedCandidateData>>>,
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
    /// 查询版本计数器 —— 单一版本域：仅会写会话的入口参与（UI 查询与热键唤醒），
    /// 只读入口（CLI/面板）不分配版本号、结果不作废。
    query_revision: Arc<AtomicU64>,
}

impl SessionDispatcher {
    /// 创建调度器。
    /// 参数：plugin_registry - 插件注册中心（注册/注销/枚举）。
    pub fn new(plugin_registry: Arc<PluginRegistry>) -> Self {
        Self {
            plugin_registry,
            enabled_plugins: DashSet::new(),

            active_session: RwLock::new(ActiveSession {
                generation: 0,
                owner: SessionOwner::Host(HostView::None),
            }),
            search_pipeline: Arc::new(RwLock::new(None)),
            candidate_pipeline: Arc::new(tokio::sync::RwLock::new(CandidatePipeline::new())),
            cached_candidates: Arc::new(RwLock::new(Arc::new(CachedCandidateData::new()))),
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
            query_revision: Arc::new(AtomicU64::new(0)),
        }
    }

    // ==================== 注册与装配 ====================

    /// 注册一个执行器；注册冲突（目标类型/动作被占用）时拒绝并记录错误，不 panic。
    pub fn register_executor(&self, executor: Arc<dyn zerolaunch_plugin_api::ActionExecutor>) {
        if let Err(e) = self.executor_registry.write().register(executor) {
            error!("执行器注册被拒绝（目标类型/动作冲突）: {}", e);
        }
    }

    /// 注销一个执行器（按 component_id）。
    pub fn unregister_executor(&self, component_id: &str) {
        self.executor_registry.write().unregister(component_id);
    }

    /// 注册一个插件（内置/第三方统一入口）。
    ///
    /// 不再建立触发词索引：路由在每次查询时按注册表元数据判定（权威来源即 `PluginMetadata`），
    /// 因此同名触发词可以并存——多个插件同时命中时按 `(priority, plugin_id)` 确定性裁决。
    /// `enabled` 为当前持久化启用状态：禁用插件仅登记不参与路由（启用恢复即改回该状态位）。
    pub fn register_plugin(
        &self,
        plugin: Arc<dyn Plugin>,
        metadata: Arc<PluginMetadata>,
        enabled: bool,
    ) {
        debug!(
            plugin_id = metadata.id.as_str(),
            mode = ?metadata.mode,
            enabled,
            "插件注册完成"
        );
        self.plugin_registry.register(plugin, metadata.clone());
        self.set_plugin_enabled_state(&metadata.id, enabled);
    }

    /// 规范化前端面板类型：第三方插件统一为 `third-party:<plugin_id>`
    /// （前端按插件 id 注册 provider）；内置插件保留自定义 panel_type。
    pub fn normalize_panel_type(
        plugin_id: &Option<String>,
        kind: PluginKind,
        panel_type: &str,
    ) -> String {
        match kind {
            PluginKind::ThirdParty => {
                format!("third-party:{}", plugin_id.as_deref().unwrap_or_default())
            }
            PluginKind::Builtin => panel_type.to_string(),
        }
    }

    /// 会话归属该插件时结束会话（注销与禁用共用：两者语义都是「该插件不再可路由」）。
    fn drop_owned_session(&self, plugin_id: &str) {
        let owned = {
            let session = self.active_session.read();
            matches!(&session.owner, SessionOwner::Plugin(identity) if identity.plugin_id == plugin_id)
        };
        if owned {
            self.reset_session(true);
        }
    }

    /// 注销一个插件：移除注册与启用状态；活动会话属于该插件时先执行会话重置。
    pub fn unregister_plugin(&self, plugin_id: &str) {
        self.plugin_registry.unregister(plugin_id);
        self.enabled_plugins.remove(plugin_id);

        self.drop_owned_session(plugin_id);
    }

    /// 写入插件级启用状态（路由与 wake_plugin 启用校验共用同一状态源）。
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

    /// 插件启用状态变更：禁用 → 该插件立即退出路由（搜索不再路由到它）并结束其会话；
    /// 启用 → 恢复路由资格。插件实例仍保留在 registry 中，不销毁。
    pub fn set_plugin_enabled(&self, plugin_id: &str, enabled: bool) {
        self.set_plugin_enabled_state(plugin_id, enabled);
        if !enabled {
            self.drop_owned_session(plugin_id);
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
        *self.cached_candidates.write() = Arc::new(candidates);
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

    /// 常驻结果框（空查询主页）是否开启 —— 决定空查询是"加载主页"还是"无会话请求"。
    /// ConfigManager 未注入（CLI/测试场景）或字段缺失时按开启处理：配置不可读不改变查询语义。
    fn is_home_enabled(&self) -> bool {
        let Some(cm) = self.config_manager() else {
            return true;
        };
        cm.get_component_setting("window-behavior-config", "is_show_home_on_empty_query")
            .and_then(|v| v.as_bool())
            .unwrap_or(true)
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

    /// 当前候选缓存世代（查询响应下发，前端确认回传校验用）。
    pub fn get_candidates_generation(&self) -> u64 {
        self.cached_candidates.read().generation()
    }

    /// 获取候选项的快照（计数 + 数据），单次锁获取保证一致性。
    pub fn get_candidates_snapshot(&self) -> (usize, Vec<zerolaunch_plugin_api::SearchCandidate>) {
        let guard = self.cached_candidates.read();
        let candidates = guard.get_candidates();
        (candidates.len(), candidates.to_vec())
    }

    /// 生成沉浸式插件候选项：启用的 Panel 形态插件 → 完整候选。
    /// keywords = 插件 trigger_keywords + 名称（Panel 形态下触发词语义为候选搜索
    /// 关键字）；图标为插件元数据 data URL（IconRequest::Data 直通图标链路）。
    pub(crate) fn build_plugin_candidates(&self) -> Vec<SearchCandidate> {
        let mut plugin_candidates = Vec::new();
        for meta in self.plugin_registry.get_all_metadata() {
            if meta.mode != PluginMode::Panel || !self.is_plugin_enabled(&meta.id) {
                continue;
            }
            let mut keywords = meta.trigger_keywords.clone();
            if !keywords.iter().any(|k| k.eq_ignore_ascii_case(&meta.name)) {
                keywords.push(meta.name.clone());
            }
            plugin_candidates.push(SearchCandidate {
                id: 0,
                name: meta.name.clone(),
                icon: IconRequest::Data(meta.icon.clone().unwrap_or_default()),
                target: ExecutionTarget::Plugin(meta.id.clone()),
                keywords,
                bias: 0.0,
                trigger_keywords: Vec::new(),
            });
        }
        plugin_candidates
    }

    /// 刷新候选项缓存。
    /// 所有触发源（定时/监控/手动/配置联动）共用本入口；刷新成功后记录时间戳，
    /// 供 auto-refresh 周期任务判断"距上次刷新是否已达间隔"（天然去重，避免重复刷新）。
    pub async fn refresh_candidates(&self) {
        let pipeline = self.candidate_pipeline.read().await;
        let candidates = pipeline.collect().await;
        let mut candidates = self.merge_plugin_candidates(candidates);
        // 全量重建后递增缓存世代：旧确认载荷（携带旧世代）在 route_confirm
        // 被拒绝，防止刷新后 id 漂移导致确认到错误候选。
        candidates.bump_generation();
        *self.cached_candidates.write() = Arc::new(candidates);
        *self.last_refresh.lock() = Some(Instant::now());
    }

    /// 将插件候选并入数据源候选缓存（启动采集与运行时刷新共用单一入口）：
    /// 不经关键字管道，仅按 target 去重。
    pub(crate) fn merge_plugin_candidates(
        &self,
        mut candidates: CachedCandidateData,
    ) -> CachedCandidateData {
        for candidate in self.build_plugin_candidates() {
            candidates.add_plugin_candidate(candidate);
        }
        candidates
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
    pub async fn debug_search(&self, query: &str) -> Vec<ScoredCandidate> {
        // 快照后释放锁再执行（远端组件经 RPC 可能耗时，不跨 await 持锁）
        let cached = self.cached_candidates.read().clone();
        let Some(pipeline) = self.search_pipeline.read().clone() else {
            return Vec::new();
        };
        let normalized = collapse_repeated_spaces(&query.to_lowercase());
        pipeline.search(&cached, &normalized).await
    }

    /// 调试用：对缓存候选项运行全量搜索（不截断 top_k），供分数分解观察。
    /// 参数：query - 原始查询文本（内部转为小写并折叠连续空格后匹配）。
    /// 返回：完整评分排序后的候选项列表；搜索管道未初始化时为空。
    pub async fn debug_search_all(&self, query: &str) -> Vec<ScoredCandidate> {
        let cached = self.cached_candidates.read().clone();
        let Some(pipeline) = self.search_pipeline.read().clone() else {
            return Vec::new();
        };
        let normalized = collapse_repeated_spaces(&query.to_lowercase());
        pipeline.search_all(&cached, &normalized).await
    }

    /// 调试用：对给定名称生成关键字列表（采集管道 DataSource 能力）。
    pub async fn debug_generate_keywords(&self, name: &str) -> Vec<String> {
        self.candidate_pipeline
            .read()
            .await
            .generate_keywords_for_name(name)
            .await
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

    /// 分配下一个查询版本号并构造门控 —— 仅会写会话的入口调用（UI 查询、热键唤醒）。
    /// 参数：无。返回：绑定当前版本号与共享计数器的门控（注入 PluginContext 供插件判断）。
    fn next_gate(&self) -> QueryRevisionGate {
        let revision = self.query_revision.fetch_add(1, Ordering::Relaxed) + 1;
        QueryRevisionGate::new(revision, self.query_revision.clone())
    }

    /// 查询过期门控：查询执行期间若有更新的同域请求（UI 查询/热键唤醒）进入后端，
    /// 本查询已过期。判定单调不可逆（过期后不会再变回最新），丢弃本次结果返回空响应
    /// 优于返回过期数据（旧 SessionRouter 语义，仅记录日志，由调用方丢弃结果）。
    /// `gate` 为 None 表示只读入口（无版本域，结果永不作废）。
    fn is_query_stale(&self, gate: Option<&QueryRevisionGate>) -> bool {
        let Some(gate) = gate else {
            return false;
        };
        if gate.is_current() {
            return false;
        }
        info!(
            query_revision = gate.revision(),
            latest_query_revision = self.query_revision.load(Ordering::Relaxed),
            site = "route",
            "查询过期，丢弃查询结果"
        );
        true
    }
    /// 查询路由裁决：返回接管的插件与派生查询词；无插件命中返回 `None`（走默认搜索）。
    ///
    /// 裁决规则（确定性，与注册表迭代顺序无关）：
    /// - 前置：**输入含空格才可能命中**（框架关键词规则与检测器的提交规则都要求空格），
    ///   无空格的输入直接跳过路由——省掉无谓的跨进程判定；
    /// - 参与资格：行内形态（Panel 形态仅经热键/候选项唤醒，不参与路由）且处于启用状态；
    /// - **统一入口**：并发调用每个候选插件的 `Plugin::match_query`（内置进程内、远端经
    ///   `plugin/match_query` RPC），整体受 `ROUTE_DEADLINE` 兜底（超时/失败按不命中处理）；
    /// - 命中者按 `priority` 小者优先、同优先级按 `plugin_id` 字典序选唯一赢家；
    /// - 查询词与输入匹配模型由赢家的触发词推导：同时满足框架关键词规则 → 取其后的剩余
    ///   （模型 `keywords`，前端可本地镜像）；否则用原始输入（模型 `custom`，前端粘性）。
    ///
    /// 并发只用于压缩耗时：裁决在结果收齐后按优先级进行，绝不按响应先后决定，避免路由不可复现。
    async fn locate_plugin(&self, raw_query: &str) -> Option<LocatedPlugin> {
        if !raw_query.contains(' ') {
            return None;
        }

        let candidates: Vec<(u32, Arc<PluginMetadata>, Arc<dyn Plugin>)> = self
            .plugin_registry
            .get_all_with_metadata()
            .into_iter()
            .filter(|(_, metadata)| {
                metadata.mode == PluginMode::Inline
                    && self.enabled_plugins.contains(metadata.id.as_str())
            })
            .map(|(plugin, metadata)| (metadata.priority, metadata, plugin))
            .collect();

        if candidates.is_empty() {
            return None;
        }

        let pending = candidates.into_iter().map(|(priority, metadata, plugin)| {
            Box::pin(async move {
                let matched = plugin
                    .match_query(raw_query, &metadata.trigger_keywords)
                    .await;
                (priority, metadata, matched)
            })
        });
        let results =
            match tokio::time::timeout(ROUTE_DEADLINE, futures_util::future::join_all(pending))
                .await
            {
                Ok(results) => results,
                Err(_) => {
                    warn!(
                        raw_query_len = raw_query.chars().count(),
                        deadline_ms = ROUTE_DEADLINE.as_millis() as u64,
                        "插件查询匹配未在截止时间内全部返回，未返回的插件按不命中处理"
                    );
                    return None;
                }
            };

        let mut hits: Vec<(u32, Arc<PluginMetadata>)> = results
            .into_iter()
            .filter(|(_, _, matched)| *matched)
            .map(|(priority, metadata, _)| (priority, metadata))
            .collect();
        hits.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.id.cmp(&b.1.id)));

        let (priority, metadata) = hits.into_iter().next()?;
        let (search_term, match_model) =
            match keyword_trigger_match(&metadata.trigger_keywords, raw_query) {
                Some(rest) => (
                    rest.to_string(),
                    InputMatch::Keywords {
                        trigger_keywords: metadata.trigger_keywords.clone(),
                    },
                ),
                None => (raw_query.to_string(), InputMatch::Custom),
            };
        debug!(
            plugin_id = metadata.id.as_str(),
            priority,
            ?match_model,
            "查询路由命中插件"
        );
        Some(LocatedPlugin {
            plugin_id: metadata.id.clone(),
            search_term,
            match_model: Some(match_model),
        })
    }

    /// 查询求值：显式插件直调（面板通道）→ 插件；触发词命中 → 插件；否则 → 默认搜索。
    ///
    /// `explicit_plugin_id` 为 Some 时跳过触发词路由，直接查询指定插件；为 None 时
    /// 走触发词路由/默认搜索。显式直调仅 `route_query_panel` 使用。
    /// `gate` 为查询版本门控：Some = 会写会话的入口（过期则丢弃结果），
    /// None = 只读入口（无版本域，结果不作废）。
    /// 返回 `Err(PluginError)` 表示插件匹配成功但处理失败（不落入默认搜索）。
    /// 本函数不产生会话副作用：投影写入由 `apply_session_projection`（仅 UI 入口调用）
    /// 承担，CLI/面板入口在结构上不具备改写会话的能力。
    #[tracing::instrument(skip(self, query), fields(trace_id = %trace_id, query_revision, owner))]
    async fn evaluate_query(
        &self,
        trace_id: &str,
        query: &Query,
        channel: QueryChannel,
        explicit_plugin_id: Option<&str>,
        gate: Option<&QueryRevisionGate>,
    ) -> Result<EvaluatedQuery, SessionDispatcherError> {
        if let Some(gate) = gate {
            tracing::Span::current().record("query_revision", gate.revision());
        }
        let (query_len, query_preview) = Self::log_query_preview(&query.raw_query);
        info!(
            query_revision = gate.map(|g| g.revision()).unwrap_or(0),
            raw_query_len = query_len,
            raw_query_preview = %query_preview,
            confirm = query.confirm,
            "查询开始"
        );

        let mut ctx = PluginContext::new(trace_id);
        // query_id = 查询追踪标识。
        ctx.with_query(trace_id.to_string());
        if let Some(gate) = gate {
            ctx.set_query_revision_gate(gate.clone());
        }
        ctx.query_channel = channel;
        ctx.locale = self.current_locale();

        // 目标插件定位：显式直调或查询路由裁决，均未命中 → 默认搜索；
        // 面板直调 search_term 为输入小写化，路由命中为关键词剥离结果或原始输入。
        let located: Option<LocatedPlugin> = match explicit_plugin_id {
            Some(pid) => Some(LocatedPlugin {
                plugin_id: pid.to_string(),
                search_term: query.raw_query.to_lowercase(),
                // 面板直调查询不参与路由裁决，无输入匹配模型（只读路径不写会话投影）。
                match_model: None,
            }),
            None => self.locate_plugin(&query.raw_query).await,
        };

        if let Some(LocatedPlugin {
            plugin_id,
            search_term,
            match_model,
        }) = located
        {
            let plugin = self.plugin_registry.get(&plugin_id).ok_or_else(|| {
                SessionDispatcherError::InvalidState(format!(
                    "{}: 插件不存在: {}",
                    if explicit_plugin_id.is_some() {
                        "面板查询"
                    } else {
                        "查询路由指向"
                    },
                    plugin_id
                ))
            })?;

            // 构造插件查询：面板直调 confirm 固定 false，触发词路由透传调用方 confirm。
            let plugin_query = Query {
                id: query.id.clone(),
                raw_query: query.raw_query.clone(),
                search_term,
                confirm: if explicit_plugin_id.is_some() {
                    false
                } else {
                    query.confirm
                },
            };
            let mut plugin_ctx = ctx.clone();
            plugin_ctx.with_plugin_id(plugin_id.clone());
            tracing::Span::current().record("owner", plugin_id.as_str());

            match plugin.query(&plugin_ctx, &plugin_query).await {
                Ok(response) => {
                    // 提交门控：查询执行期间若有更新的同域请求进入后端，本查询已过期
                    // （判定单调，过期后不会再变回最新），直接丢弃本次结果返回空响应。
                    if self.is_query_stale(gate) {
                        return Ok(EvaluatedQuery {
                            response: QueryResponse::Empty,
                            owner: Some(plugin_id),
                            current: false,
                            match_model,
                        });
                    }
                    info!(
                        query_revision = gate.map(|g| g.revision()).unwrap_or(0),
                        target = %plugin_id,
                        "路由命中插件"
                    );
                    Ok(EvaluatedQuery {
                        response,
                        owner: Some(plugin_id),
                        current: true,
                        match_model,
                    })
                }
                Err(e) => {
                    // 插件匹配成功但处理失败：不静默切换默认搜索，沿 IPC 错误通道上报。
                    error!(
                        query_revision = gate.map(|g| g.revision()).unwrap_or(0),
                        target = %plugin_id,
                        error = %e,
                        "插件查询执行失败"
                    );
                    Err(SessionDispatcherError::PluginError(e.to_string()))
                }
            }
        } else {
            // 默认搜索：搜索管道 + 行内参数检测 + ListItem 映射。
            // 快照后释放锁再执行（远端引擎/增强器经 RPC 可能耗时，不跨 await 持锁）
            let cached = self.cached_candidates.read().clone();
            let Some(pipeline) = self.search_pipeline.read().clone() else {
                warn!("SearchPipeline 未初始化，返回空结果");
                // 求值未就绪不构成有效交互：不产生会话投影（与既有行为一致）。
                return Ok(EvaluatedQuery {
                    response: QueryResponse::Empty,
                    owner: None,
                    current: false,
                    match_model: None,
                });
            };
            let normalized = collapse_repeated_spaces(&query.search_term);
            let scored_candidates = pipeline.search(&cached, &normalized).await;

            // 提交门控（与插件分支一致）：搜索计算期间若有更新的同域请求进入后端，
            // 本查询已过期，丢弃结果——过期返回空结果优于返回过期数据（并发/排队
            // 场景），同时避免过期结果被入口写入投影。
            if self.is_query_stale(gate) {
                return Ok(EvaluatedQuery {
                    response: QueryResponse::Empty,
                    owner: None,
                    current: false,
                    match_model: None,
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
                        return Ok(EvaluatedQuery {
                            response: QueryResponse::InlineParam {
                                candidate_id: sc.id,
                                trigger_keyword: trimmed.to_string(),
                                user_arg_count,
                            },
                            owner: None,
                            current: true,
                            match_model: None,
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
                    // 动作列表统一来自 ExecutorRegistry：插件候选由宿主内置
                    // PluginWakeExecutor 提供默认「打开」动作。
                    let actions = self
                        .executor_registry
                        .read()
                        .get_actions(search_candidate.target.target_type());
                    // 副标题：插件候选展示插件描述，描述缺失时兜底插件 id
                    let subtitle = match &search_candidate.target {
                        ExecutionTarget::Plugin(plugin_id) => self
                            .plugin_registry
                            .get_metadata(plugin_id)
                            .map(|m| m.description.clone())
                            .filter(|d| !d.is_empty())
                            .unwrap_or_else(|| format!("plugin id: {}", plugin_id)),
                        _ => search_candidate.target.payload().to_string(),
                    };
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
                        subtitle,
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

            Ok(EvaluatedQuery {
                response: QueryResponse::List { results },
                owner: None,
                current: true,
                match_model: None,
            })
        }
    }

    /// 组装路由响应：`current == false`（过期/未就绪）时 `evaluate_query` 已给出空响应；
    /// 会话代际与归属照常回填（过期丢弃仍回填插件 id，供 Inspector 归属可观测）。
    fn routed_from(&self, evaluated: EvaluatedQuery) -> RoutedQuery {
        RoutedQuery {
            response: evaluated.response,
            generation: self.current_generation(),
            plugin_id: evaluated.owner,
        }
    }

    /// 按求值结果写入会话投影 —— 查询链路上唯一允许改写会话状态的函数。
    /// 调用方保证仅在 `evaluated.current == true` 时调用（过期/未就绪不写投影）。
    fn apply_session_projection(&self, evaluated: &EvaluatedQuery) {
        if let Some(plugin_id) = &evaluated.owner {
            // 展示形态：keep_search_bar 决定行内/全页面；非面板响应按行内形态进入。
            let view = match &evaluated.response {
                QueryResponse::CustomPanel {
                    keep_search_bar, ..
                } => {
                    if *keep_search_bar {
                        PluginView::Panel
                    } else {
                        PluginView::Immersive
                    }
                }
                _ => PluginView::Panel,
            };
            // 插件面板命中即进入插件会话投影：投递语义（无条件推送，语义见 deliver_plugin_session）。
            self.deliver_plugin_session(
                PluginIdentity {
                    plugin_id: plugin_id.clone(),
                    view,
                    input_match: evaluated.match_model.clone(),
                },
                None,
            );
            return;
        }
        match &evaluated.response {
            QueryResponse::InlineParam { candidate_id, .. } => {
                *self.search_state.write() = SearchSubState::InlineParam {
                    candidate_id: *candidate_id,
                };
                self.enter_host_session(HostView::InlineParam);
            }
            _ => {
                *self.search_state.write() = SearchSubState::Search;
                self.enter_host_session(HostView::Search);
            }
        }
    }

    /// UI 查询入口（搜索栏输入）——唯一允许改写会话状态的查询入口。
    /// 两条路径：空查询且关闭常驻结果框 → 结束会话；其余 → 求值后写入会话投影。
    /// 参数：trace_id - 追踪标识；query - 查询。
    /// 返回：路由响应（含会话代际与归属）。
    pub async fn route_query_ui(
        &self,
        trace_id: &str,
        query: &Query,
    ) -> Result<RoutedQuery, SessionDispatcherError> {
        // 版本门控先分配：空查询同样取代在途查询（慢响应不得覆盖退出后的投影）。
        let gate = self.next_gate();
        // 空查询 = 前端的"无会话"请求：关闭常驻结果框时无可展示的会话内容，
        // 直接结束会话（含默认搜索子状态清理），不进入搜索管道。
        // 前端因此无需在退出/回退路径上显式声明会话结束。
        if query.search_term.trim().is_empty() && !self.is_home_enabled() {
            self.reset_session(true);
            return Ok(RoutedQuery {
                response: QueryResponse::Empty,
                generation: self.current_generation(),
                plugin_id: None,
            });
        }
        let evaluated = self
            .evaluate_query(trace_id, query, QueryChannel::Ui, None, Some(&gate))
            .await?;
        if evaluated.current {
            self.apply_session_projection(&evaluated);
        }
        Ok(self.routed_from(evaluated))
    }

    /// CLI 查询入口（/v1/query）——只读辅助路径，不改写会话状态、结果不作废。
    /// 参数：trace_id - 追踪标识；query - 查询。返回：路由响应。
    pub async fn route_query_cli(
        &self,
        trace_id: &str,
        query: &Query,
    ) -> Result<RoutedQuery, SessionDispatcherError> {
        let evaluated = self
            .evaluate_query(trace_id, query, QueryChannel::Cli, None, None)
            .await?;
        Ok(self.routed_from(evaluated))
    }

    /// 面板查询入口（面板内 bridge_query 显式指定插件）——只读辅助路径，
    /// 不改写会话状态、结果不作废。
    /// 参数：trace_id - 追踪标识；query - 查询；plugin_id - 目标插件。
    /// 返回：路由响应；插件不存在时返回 `InvalidState`。
    pub async fn route_query_panel(
        &self,
        trace_id: &str,
        query: &Query,
        plugin_id: &str,
    ) -> Result<RoutedQuery, SessionDispatcherError> {
        let evaluated = self
            .evaluate_query(trace_id, query, QueryChannel::Panel, Some(plugin_id), None)
            .await?;
        Ok(self.routed_from(evaluated))
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
        match &session.owner {
            SessionOwner::Plugin(identity) => {
                let plugin_id = &identity.plugin_id;
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
            SessionOwner::Host(_) => {
                // 默认搜索只处理宿主候选确认；插件面板动作在插件归属分支处理。
                let ConfirmRequest::Candidate {
                    candidate_id,
                    candidate_generation,
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
                            .execute_candidate(
                                candidate_id,
                                candidate_generation,
                                &action_id,
                                &query_text,
                                &user_args,
                            )
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
                            self.enter_host_session(HostView::ParamPanel);
                            return Ok(RoutedConfirm {
                                outcome: ConfirmOutcome::EnterParamPanel {
                                    candidate_id,
                                    user_arg_count,
                                },
                                generation: self.current_generation(),
                            });
                        }
                        match self
                            .execute_candidate(
                                candidate_id,
                                candidate_generation,
                                &action_id,
                                &query_text,
                                &user_args,
                            )
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

    /// 共享骨架：读取并克隆活动会话，校验会话未结束与请求代际一致。
    /// 参数：request_generation - 请求携带的代际。
    /// 返回：校验通过的活动会话快照（确认入口共用）。
    fn active_session_checked(
        &self,
        request_generation: u64,
    ) -> Result<ActiveSession, SessionDispatcherError> {
        let session = self.active_session.read().clone();
        if session.owner.is_ended() {
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
        candidate_generation: u64,
        action_id: &str,
        query_text: &str,
        user_args: &[String],
    ) -> Result<(), ConfirmError> {
        // 候选世代校验：缓存刷新（60s 定时/安装监控/配置变更）后 id 重排，
        // 旧世代确认载荷可能指向错误候选，直接拒绝。
        {
            let cached = self.cached_candidates.read();
            if candidate_generation != 0 && cached.generation() != candidate_generation {
                return Err(ConfirmError(format!(
                    "候选缓存已刷新（世代 {} != {}），请重试",
                    cached.generation(),
                    candidate_generation
                )));
            }
        }
        // 候选快照后释放锁（record 对远端增强器经 RPC，不跨 await 持锁）
        let (exec_ctx, candidate_snapshot) = {
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
            (exec_ctx, cached.clone())
        };
        // 记录搜索行为：通知式 fire-and-forget（远端增强器 record RPC 超时 2s×N，
        // 不阻塞执行器解析与执行；失败仅告警，见 RemoteComponent::record）。
        let pipeline = {
            let guard = self.search_pipeline.read();
            guard.clone()
        };
        if let Some(pipeline) = pipeline {
            let query_text = query_text.to_string();
            let config_manager = self.config_manager();
            tauri::async_runtime::spawn(async move {
                pipeline
                    .record(candidate_id, candidate_snapshot.as_ref(), &query_text)
                    .await;
                // 记录后立即落盘组件运行态（启动历史/查询亲和），与用户配置分离，
                // 避免重启后统计归零；未注入 ConfigManager（CLI 场景）时跳过。
                if let Some(config_manager) = config_manager {
                    config_manager.flush_runtime_state();
                }
            });
        }
        // 锁在 await 前已全部释放；插件候选（ExecutionTarget::Plugin）由宿主内置
        // PluginWakeExecutor 处理，统一经本管道。
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

    /// 进入宿主会话投影（变更通知语义）：形态变化时递增代际并更新活动会话；
    /// 投影未变则不推送。
    ///
    /// 适用：载荷随 `bridge_query` 响应下发的宿主形态（默认搜索 / 行内参数 / 参数面板）——
    /// 前端渲染这些形态不依赖事件投递。
    fn enter_host_session(&self, view: HostView) {
        self.enter_session_inner(SessionOwner::Host(view), None, false);
    }

    /// 投递插件会话投影（投递语义）：无条件推送，不按投影变化裁剪。
    ///
    /// 不变式：插件的归属/交互契约/输入匹配模型**只能**经事件送达前端（查询响应不含这些字段），
    /// 而前端可在不发任何 IPC 的情况下本地退出面板（后端无从观测），故"投影未变"
    /// 不能作为"前端已持有交互契约"的依据——命中路径必须每次投递，否则同面板重入会
    /// 丢失交互契约（Escape 等按键失效）。此处不得按投影变化"优化"为变更通知。
    ///
    /// 调用方：① UI 查询命中插件（`content` 为 None，载荷随查询响应下发）；
    /// ② 热键唤醒（`content` 是唤醒路径唯一的载荷通道）。
    fn deliver_plugin_session(
        &self,
        identity: PluginIdentity,
        content: Option<PluginPanelContent>,
    ) {
        self.enter_session_inner(SessionOwner::Plugin(identity), content, true);
    }

    /// 会话投影写入内部实现：`content` 为唤醒路径的面板渲染载荷，`deliver` 为投递语义
    /// （无条件推送，语义见 `deliver_plugin_session`）。
    fn enter_session_inner(
        &self,
        owner: SessionOwner,
        content: Option<PluginPanelContent>,
        deliver: bool,
    ) {
        debug_assert!(
            content.is_none() || matches!(owner, SessionOwner::Plugin(_)),
            "面板渲染载荷只属于插件会话"
        );
        let mut session = self.active_session.write();
        let changed = session.owner != owner;
        if !changed && !deliver {
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
                owner: owner.clone(),
            };
        }
        drop(session);
        self.push_session_state(generation, &owner, content);
    }

    /// 推送会话状态事件（无 emitter 的 CLI 场景直接跳过）。
    ///
    /// 事件载荷按 `owner` 分派：宿主会话不带插件字段；插件会话在投递时解析交互契约
    /// （不入会话身份：配置变更需即时生效，且不参与代际比较）。
    fn push_session_state(
        &self,
        generation: u64,
        owner: &SessionOwner,
        content: Option<PluginPanelContent>,
    ) {
        let Some(emitter) = self.session_emitter.read().clone() else {
            return;
        };
        let event = match owner {
            SessionOwner::Host(view) => SessionStateEvent::Host {
                generation,
                view: *view,
            },
            SessionOwner::Plugin(identity) => SessionStateEvent::Plugin {
                generation,
                identity: identity.clone(),
                interaction: self.resolve_interaction(identity),
                panel_content: content.map(Box::new),
            },
        };
        emitter(event);
    }

    /// 解析插件交互契约 —— 注册中心查不到时按宿主默认键降级并告警。
    ///
    /// 该状态可由用户操作稳定到达（卸载/禁用与在途查询并发：查询返回后写入投影时插件已注销），
    /// 故只降级不 panic。
    fn resolve_interaction(&self, identity: &PluginIdentity) -> PanelInteraction {
        match self.plugin_registry.get(&identity.plugin_id) {
            Some(plugin) => plugin.interaction_policy(),
            None => {
                warn!(
                    plugin_id = identity.plugin_id.as_str(),
                    "会话归属的插件不在注册中心，交互契约按宿主默认降级"
                );
                PanelInteraction::default()
            }
        }
    }

    /// 会话重置：参数面板/行内参数/搜索恒重置；插件模式仅当 `reset_plugins` 为 true 时重置
    /// （支持隐藏/显示间保持插件面板状态）。返回 true 表示实际执行了重置。
    pub fn reset_session(&self, reset_plugins: bool) -> bool {
        let mut session = self.active_session.write();
        let should_reset = match &session.owner {
            SessionOwner::Plugin(_) => reset_plugins,
            SessionOwner::Host(view) => *view != HostView::None,
        };
        if !should_reset {
            return false;
        }
        let changed = !session.owner.is_ended();
        let generation = if changed {
            session.generation + 1
        } else {
            session.generation
        };
        if changed {
            *session = ActiveSession {
                generation,
                owner: SessionOwner::Host(HostView::None),
            };
        }
        // 默认搜索子状态重置（InlineParam/ParamPanel 属本调度器内嵌状态；
        // 插件面板状态由插件自己管理，宿主不感知）。
        *self.search_state.write() = SearchSubState::Search;
        *self.parameter_snapshot.lock() = ParameterSnapshot::empty();
        drop(session);
        // 会话结束投影：唯一事件通道推送（原 session-reset 事件已删除）。
        if changed {
            self.push_session_state(generation, &SessionOwner::Host(HostView::None), None);
        }
        true
    }

    /// 当前活动会话（克隆）。
    pub fn current_session(&self) -> ActiveSession {
        self.active_session.read().clone()
    }

    /// 当前会话形态的 snake_case 词（CLI `/v1/session` 等只读场景）。
    pub fn current_view_str(&self) -> &'static str {
        // 形态词是 'static：单次加锁读数后守卫即释放，无非原子多次读取。
        let view = {
            let session = self.active_session.read();
            session.owner.view_str()
        };
        view
    }

    /// 当前会话代际。
    pub fn current_generation(&self) -> u64 {
        self.active_session.read().generation
    }

    /// 重新推送当前会话投影（配置变更后调用，面板内调整防抖等即时生效）。
    pub fn reemit_current_session(&self) {
        let session = self.active_session.read().clone();
        if session.owner.is_ended() {
            return;
        }
        self.push_session_state(session.generation, &session.owner, None);
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

    /// 热键唤醒插件（独立插件）：捕获参数快照 → 空查询 → 进入全页面接管会话。
    /// 响应必须为 CustomPanel 且 keep_search_bar=false（全页面接管契约；keep_search_bar=true
    /// 属违约：debug 构建 panic 暴露、release 构建按声明形态降级为 PluginPanel 正常唤醒）；
    /// 载荷经会话事件 panelContent 一并推送（窗口隐藏时前端无查询响应可依赖）。
    /// 非 CustomPanel 响应（List/Empty）属契约违约，返回错误（前端无载荷可渲染，
    /// 静默进入会导致前后端投影失步）。
    /// 唤醒与 UI 查询共用版本计数器：唤醒开始即递增使在途 UI 查询过期，查询返回后
    /// 再次校验——期间若有更新的查询/唤醒进入则本唤醒已过期，丢弃结果不进入会话
    /// （与 evaluate_query 提交流程同构），防止慢唤醒覆盖用户等待期间发起的新会话。
    pub async fn wake_plugin(&self, plugin_id: &str) -> Result<(), SessionDispatcherError> {
        // 启用校验：禁用插件不可被热键唤醒（前端热键表可能残留过期条目，
        // 后端为权威裁决，与触发词路由的「禁用即不路由」语义一致）。
        if !self.is_plugin_enabled(plugin_id) {
            return Err(SessionDispatcherError::InvalidState(format!(
                "热键唤醒的插件未启用: {}",
                plugin_id
            )));
        }
        // 形态校验：仅 panel 形态插件可热键唤醒（后端权威裁决，行内插件即使声明 hotkey 也被拒绝）
        let meta = self.plugin_registry.get_metadata(plugin_id);
        if let Some(ref meta) = meta {
            if meta.mode != PluginMode::Panel {
                return Err(SessionDispatcherError::InvalidState(format!(
                    "热键唤醒的插件 {} 为行内形态（mode=inline），仅 panel 形态插件可热键唤醒",
                    plugin_id
                )));
            }
        }
        // 版本门控：唤醒与会写会话的查询共用同一版本域——占用版本号使在途 UI 查询过期，
        // 查询返回后校验自身是否仍为最新，过期则丢弃（不写快照、不进入会话）。
        let gate = self.next_gate();

        let host_api: Arc<HostApi> = self.host_api.read().clone().ok_or_else(|| {
            SessionDispatcherError::NotInitialized(
                "HostApi not initialized in SessionDispatcher".to_string(),
            )
        })?;
        let snapshot = host_api.capture_parameter_snapshot().await;

        let plugin = self.plugin_registry.get(plugin_id).ok_or_else(|| {
            SessionDispatcherError::InvalidState(format!("热键唤醒的插件不存在: {}", plugin_id))
        })?;

        let trace_id = crate::utils::trace_id::generate_trace_id();
        let mut ctx = PluginContext::new(&trace_id);
        ctx.with_query(trace_id.clone());
        ctx.with_plugin_id(plugin_id.to_string());
        ctx.locale = self.current_locale();
        ctx.set_query_revision_gate(gate.clone());
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

        // 提交门控：查询期间若有更新的同域请求进入后端，本唤醒已过期，
        // 丢弃结果返回成功（前端保持用户最新会话，不推送覆盖事件）。
        if self.is_query_stale(Some(&gate)) {
            return Ok(());
        }
        *self.parameter_snapshot.lock() = snapshot;

        // 展示形态与载荷：热键唤醒默认 = 独立插件 = 全页面接管（PluginImmersive）。
        // keep_search_bar=true（行内面板）与热键唤醒契约冲突：debug 构建用 debug_assert
        // 强制 panic 暴露（契约违约即宿主逻辑缺陷，快速定位）；release 构建正常运行——
        // 按插件声明形态降级为 PluginPanel（保留搜索栏），与 apply_session_projection 的
        // keep_search_bar → 展示形态映射保持一致，不因插件违约而中止唤醒。
        // 非 CustomPanel 响应（List/Empty）属契约违约，返回错误（前端无载荷可渲染，
        // 静默进入会导致前后端投影失步）。
        let (view, content) = match response {
            QueryResponse::CustomPanel {
                panel_type,
                data,
                actions,
                keep_search_bar,
            } => {
                debug_assert!(
                    !keep_search_bar,
                    "热键唤醒插件 {} 返回 keep_search_bar=true（行内面板）——热键唤醒仅支持全页面接管（PluginImmersive），插件契约违约",
                    plugin_id
                );
                let view = if keep_search_bar {
                    PluginView::Panel
                } else {
                    PluginView::Immersive
                };
                // 第三方插件 panel_type 统一为 third-party:<id>（前端 provider 匹配契约）
                let normalized = Self::normalize_panel_type(
                    &Some(plugin_id.to_string()),
                    meta.as_ref()
                        .map(|m| m.kind)
                        .unwrap_or(PluginKind::ThirdParty),
                    &panel_type,
                );
                (
                    view,
                    Some(PluginPanelContent {
                        panel_type: normalized,
                        data,
                        actions: actions.into_iter().map(ResultActionDto::from).collect(),
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
            presentation = view.as_str(),
            "热键唤醒插件"
        );
        // 热键唤醒无路由裁决：按插件声明的触发词下发前端镜像谓词（声明为空则无谓词）。
        let trigger_keywords = meta
            .as_ref()
            .map(|m| m.trigger_keywords.clone())
            .unwrap_or_default();
        let input_match =
            (!trigger_keywords.is_empty()).then_some(InputMatch::Keywords { trigger_keywords });
        self.deliver_plugin_session(
            PluginIdentity {
                plugin_id: plugin_id.to_string(),
                view,
                input_match,
            },
            content,
        );
        // 成功唤醒后统一确保窗口可见（热键与候选项确认两条唤醒路径共用；
        // show_window 幂等，窗口已可见时无副作用）。
        host_api.show_window().await;
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
        // build_search_pipeline 恒返回 Some：无引擎时返回透传管道（候选零分，仅增强器排序）。
        match self.components.build_search_pipeline(&cm, top_k) {
            Some(pipeline) => {
                info!("搜索管道已重建 (top_k: {})", pipeline.top_k());
                *self.search_pipeline.write() = Some(pipeline);
            }
            None => {
                // 防御分支：正常情况下不可达（build 恒 Some）。
                warn!("搜索管道重建返回 None，保留原管道");
            }
        }
    }

    /// 引擎互斥不变量：仅允许一个启用的搜索引擎。保持 keep_id 启用，禁用其他启用的引擎
    /// （经 set_enabled 持久化并发布事件，各入口统一生效；disable 事件不会再走互斥，无递归）。
    fn enforce_single_search_engine(&self, keep_id: &str) {
        let Some(cm) = self.config_manager() else {
            return;
        };
        let others: Vec<String> = self
            .components
            .search_engine_ids()
            .into_iter()
            .filter(|id| id != keep_id && cm.is_enabled(id))
            .collect();
        if !others.is_empty() {
            warn!(
                "SearchEngine 互斥：启用 {} 时自动禁用 {}",
                keep_id,
                others.join(", ")
            );
            for id in others {
                let _ = cm.set_enabled(&id, false);
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
                    ComponentType::SearchEngine => {
                        if *enabled {
                            // 引擎互斥（后端权威不变量）：启用新引擎时自动禁用其他启用的引擎，
                            // 任意入口（UI toggle / CLI / 配置导入）启用都经事件统一生效。
                            self.enforce_single_search_engine(component_id);
                        }
                        info!("搜索引擎启用状态变更，重建搜索管道");
                        self.rebuild_search_pipeline();
                    }
                    ComponentType::ScoreBooster => {
                        info!("分数增强器启用状态变更，重建搜索管道");
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
                    if let Some(engine) = comp.clone().as_search_engine() {
                        self.components.register_search_engine(engine);
                    }
                    if let Some(booster) = comp.clone().as_score_booster() {
                        self.components.register_score_booster(booster);
                    }
                    if let Some(optimizer) = comp.clone().as_keyword_optimizer() {
                        self.components.register_keyword_optimizer(optimizer);
                    }
                    if let Some(injector) = comp.clone().as_keyword_injector() {
                        self.components.register_keyword_injector(injector);
                    }
                    if let Some(p) = comp.clone().as_plugin() {
                        // 按组件持久化启用状态决定是否建立触发词路由（禁用插件重启后不路由）
                        let enabled = self
                            .config_manager()
                            .map(|cm| cm.is_enabled(comp.core.component_id()))
                            .unwrap_or(true);
                        self.register_plugin(p.clone(), adapters.metadata.clone(), enabled);
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
                // 重建搜索管道以包含新引擎/增强器
                self.rebuild_search_pipeline();
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
                    if comp.is_search_engine() {
                        self.components
                            .unregister_search_engine(comp.core.component_id());
                    }
                    if comp.is_score_booster() {
                        self.components
                            .unregister_score_booster(comp.core.component_id());
                    }
                    if comp.is_keyword_optimizer() {
                        self.components
                            .unregister_keyword_optimizer(comp.core.component_id());
                    }
                    if comp.is_keyword_injector() {
                        self.components
                            .unregister_keyword_injector(comp.core.component_id());
                    }
                }
                // 重建候选管道以移除已解注册的组件
                self.rebuild_candidate_pipeline().await;
                // 重建搜索管道以移除已解注册的引擎/增强器
                self.rebuild_search_pipeline();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::setting_builders::SchemaBuilder;
    use async_trait::async_trait;
    use zerolaunch_plugin_api::config::{
        ComponentCore, ComponentType, Configurable, SettingDefinition,
    };
    use zerolaunch_plugin_api::mock::helpers::mock_platform_services;
    use zerolaunch_plugin_api::mock::*;
    use zerolaunch_plugin_api::plugin::{PanelInteraction, PanelKeyAction, PanelKeyBinding};
    use zerolaunch_plugin_api::services::resource::AppResourceService;
    use zerolaunch_plugin_api::services::storage::storage_service::StorageService;
    use zerolaunch_plugin_api::services::theme::{Theme, ThemeProvider};
    use zerolaunch_plugin_api::services::timer::TokioTimerManager;
    use zerolaunch_plugin_api::{
        HostApiError, PluginError, PluginHandle, PluginKind, PluginMetadata, PluginMode,
    };

    /// 测试主题提供器，固定返回浅色主题。
    struct StubThemeProvider;

    impl ThemeProvider for StubThemeProvider {
        /// 返回固定浅色主题供测试 HostApi 构造。
        fn current_system_theme(&self) -> Result<Theme, HostApiError> {
            Ok(Theme::Light)
        }
    }

    /// 构建仅含桩组件的 HostApi（测试专用，不触达真实平台能力）。
    /// 平台端口用 mock_platform_services() 全桩构造（主题提供器替换为本地固定浅色桩）。
    fn test_host_api() -> Arc<HostApi> {
        let storage: Arc<dyn StorageService> = Arc::new(StubStorageService);
        let mut platform = mock_platform_services();
        platform.theme_provider = Arc::new(StubThemeProvider);
        let api = HostApi::builder("mock_icons".to_string())
            .platform(platform)
            .parameter_resolver(Arc::new(StubParameterResolver))
            .timer_manager(Arc::new(TokioTimerManager::new()))
            .storage_service(storage)
            .app_resource(Arc::new(AppResourceService::new("mock_icons".to_string())))
            .model_service(Arc::new(StubModelService))
            .notify_callback(|_, _| {})
            .hide_window_callback(|| {})
            .show_window_callback(|| {})
            .is_window_visible_callback(|| false)
            .set_window_position_callback(|_, _| {})
            .build()
            .expect("构建测试 HostApi 失败");
        Arc::new(api)
    }

    /// 触发词路由测试用最小插件桩 —— 仅填充元数据（触发词）与匹配裁决，其余方法空实现。
    /// 避免测试模块引用内置实现（plugin_framework 层不得依赖 builtin_plugin，P3 层级）。
    struct TriggerStubPlugin {
        /// 插件级元数据：`trigger_keywords` 决定框架关键词判定的输入。
        metadata: PluginMetadata,
        /// 组件级身份。
        core: ComponentCore,
        /// 自定义匹配结果：None = 不覆盖（跑与生产默认实现同一份关键词判定）；Some = 覆盖返回值。
        custom_verdict: Option<bool>,
    }

    impl TriggerStubPlugin {
        /// 该桩的插件级元数据副本（注册中心条目数据源）。
        fn metadata_arc(&self) -> Arc<PluginMetadata> {
            Arc::new(self.metadata.clone())
        }

        fn with_trigger(trigger: &str) -> Self {
            Self::with_trigger_and_id(trigger, &format!("test.{}", trigger))
        }

        /// 以 panel 形态（独立插件）构造：query 返回 Empty（非 CustomPanel），
        /// 供热键唤醒契约违约路径测试（mode 校验通过后仍会因响应非 CustomPanel 被拒）。
        fn with_panel_trigger(trigger: &str) -> Self {
            let mut plugin = Self::with_trigger(trigger);
            plugin.metadata.mode = PluginMode::Panel;
            plugin
        }

        /// 以 inline 形态 + 声明热键构造：热键唤醒应被 mode 校验拒绝（行内插件不可热键唤醒）。
        fn with_inline_hotkey_trigger(trigger: &str) -> Self {
            let mut plugin = Self::with_trigger(trigger);
            plugin.metadata.hotkey = Some("Ctrl+E".to_string());
            plugin
        }

        /// 指定插件 id 的构造器：允许两个插件声明相同触发词（用于并存与优先级裁决测试）。
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
                    icon: None,
                    mode: PluginMode::Inline,
                },
                core: ComponentCore::new(
                    id.to_string(),
                    "测试桩".to_string(),
                    "触发词路由测试".to_string(),
                    ComponentType::Plugin,
                    0,
                ),
                custom_verdict: None,
            }
        }

        /// 自定义匹配桩：`match_query` 恒返回指定结果（触发词仍声明，用于验证
        /// "覆盖后由插件自行判定，不再走框架关键词规则"）。
        fn with_custom_matcher(trigger: &str, matched: bool) -> Self {
            let mut plugin = Self::with_trigger(trigger);
            plugin.custom_verdict = Some(matched);
            plugin
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

        /// 查询匹配：未指定自定义结果时，按生产默认实现同一份共享关键词判定作答。
        async fn match_query(&self, raw_query: &str, keywords: &[String]) -> bool {
            self.custom_verdict
                .unwrap_or_else(|| keyword_trigger_match(keywords, raw_query).is_some())
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

    /// 关键词模型插件注册后参与路由：触发词 + 空格命中并切出查询词；无空格不命中。
    /// 回归：此前 bootstrap 只调 plugin_registry().register（不参与路由），
    /// 导致内置触发式插件（translator/calculator）路由恒 miss、静默落入默认搜索。
    #[tokio::test]
    async fn keyword_model_plugin_routes_on_trigger_with_space() {
        let dispatcher = SessionDispatcher::new(Arc::new(PluginRegistry::new()));
        let plugin = Arc::new(TriggerStubPlugin::with_trigger("="));
        dispatcher.register_plugin(plugin.clone(), plugin.metadata_arc(), true);

        let hit = dispatcher
            .locate_plugin("= 1+1")
            .await
            .expect("触发词 + 空格应命中");
        assert_eq!(hit.plugin_id, "test.=");
        assert_eq!(hit.search_term, "1+1");

        // 无空格分隔 → 不命中（与前端关键词镜像判定一致）
        assert!(dispatcher.locate_plugin("=1+1").await.is_none());
    }

    /// 禁用插件后不再参与路由；重新启用后恢复。对未注册插件启用无害。
    /// 回归：config_set_enabled 对 Plugin 类型组件曾「无需响应」，禁用后插件仍可路由使用。
    #[tokio::test]
    async fn set_plugin_enabled_toggles_routing() {
        let dispatcher = SessionDispatcher::new(Arc::new(PluginRegistry::new()));
        let plugin = Arc::new(TriggerStubPlugin::with_trigger("="));
        let plugin_id = plugin.metadata.id.clone();
        dispatcher.register_plugin(plugin.clone(), plugin.metadata_arc(), true);
        assert!(dispatcher.locate_plugin("= 1+1").await.is_some());

        dispatcher.set_plugin_enabled(&plugin_id, false);
        assert!(dispatcher.locate_plugin("= 1+1").await.is_none());

        dispatcher.set_plugin_enabled(&plugin_id, true);
        assert!(dispatcher.locate_plugin("= 1+1").await.is_some());

        // 对未注册插件启用：无害（无路由资格可恢复）
        dispatcher.set_plugin_enabled("not-registered", true);
        assert!(dispatcher.locate_plugin("= 1+1").await.is_some());
    }

    /// 持久化为禁用的插件注册时不参与路由（重启后保持禁用语义）。
    #[tokio::test]
    async fn register_disabled_plugin_does_not_route() {
        let dispatcher = SessionDispatcher::new(Arc::new(PluginRegistry::new()));
        let plugin = Arc::new(TriggerStubPlugin::with_trigger("="));
        let plugin_id = plugin.metadata.id.clone();
        dispatcher.register_plugin(plugin.clone(), plugin.metadata_arc(), false);

        assert!(dispatcher.locate_plugin("= 1+1").await.is_none());

        dispatcher.set_plugin_enabled(&plugin_id, true);
        assert!(dispatcher.locate_plugin("= 1+1").await.is_some());
    }

    /// 同名触发词并存不再被拒绝：多个插件同时命中时按 (priority, plugin_id) 确定性裁决，
    /// 胜者禁用后次优先者接管（不因并存而失效）。
    #[tokio::test]
    async fn same_keyword_coexists_and_priority_decides() {
        let dispatcher = SessionDispatcher::new(Arc::new(PluginRegistry::new()));
        let mut plugin_a = TriggerStubPlugin::with_trigger_and_id("=", "plugin-a");
        plugin_a.metadata.priority = 10;
        let plugin_a = Arc::new(plugin_a);
        let mut plugin_b = TriggerStubPlugin::with_trigger_and_id("=", "plugin-b");
        plugin_b.metadata.priority = 20;
        let plugin_b = Arc::new(plugin_b);

        dispatcher.register_plugin(plugin_a.clone(), plugin_a.metadata_arc(), true);
        dispatcher.register_plugin(plugin_b.clone(), plugin_b.metadata_arc(), true);

        // priority 小者胜（与注册顺序、迭代顺序无关）
        assert_eq!(
            dispatcher
                .locate_plugin("= 1+1")
                .await
                .expect("两个插件都应命中")
                .plugin_id,
            "plugin-a"
        );

        dispatcher.set_plugin_enabled("plugin-a", false);
        assert_eq!(
            dispatcher
                .locate_plugin("= 1+1")
                .await
                .expect("次优先者接管")
                .plugin_id,
            "plugin-b"
        );

        dispatcher.set_plugin_enabled("plugin-b", false);
        assert!(dispatcher.locate_plugin("= 1+1").await.is_none());
    }

    /// 自定义匹配模型：声明 Custom 的插件由 `match_query` 裁决，不参与宿主关键词兜底；
    /// 命中时查询词为原始输入（不剥离触发词）；与关键词插件同时命中时同样按优先级裁决。
    #[tokio::test]
    async fn custom_matcher_decides_and_competes_by_priority() {
        let dispatcher = SessionDispatcher::new(Arc::new(PluginRegistry::new()));

        // 覆盖判定返回 false：即使声明了触发词也不命中（覆盖后框架关键词规则不再参与）
        let custom_miss = Arc::new(TriggerStubPlugin::with_custom_matcher("path", false));
        dispatcher.register_plugin(custom_miss.clone(), custom_miss.metadata_arc(), true);
        assert!(dispatcher.locate_plugin("path C:\\Users").await.is_none());
        dispatcher.set_plugin_enabled("test.path", false);

        // 覆盖判定命中 + 仍声明触发词：查询词按框架规则剥离触发词（模型 keywords）
        let mut custom_hit = TriggerStubPlugin::with_custom_matcher("path", true);
        custom_hit.metadata.id = "custom-path".to_string();
        custom_hit.metadata.priority = 50;
        let custom_hit = Arc::new(custom_hit);
        dispatcher.register_plugin(custom_hit.clone(), custom_hit.metadata_arc(), true);

        let hit = dispatcher
            .locate_plugin("path C:\\Users")
            .await
            .expect("自定义匹配应接管");
        assert_eq!(hit.plugin_id, "custom-path");
        assert_eq!(hit.search_term, "C:\\Users");
        assert_eq!(
            hit.match_model,
            Some(InputMatch::Keywords {
                trigger_keywords: vec!["path".to_string()]
            })
        );

        // 覆盖判定命中 + 无触发词（检测器形态）：查询词为原始输入（模型 custom）
        let mut detector_like = TriggerStubPlugin::with_custom_matcher("url", true);
        detector_like.metadata.id = "detector-like".to_string();
        detector_like.metadata.priority = 1;
        detector_like.metadata.trigger_keywords.clear();
        let detector_like = Arc::new(detector_like);
        dispatcher.register_plugin(detector_like.clone(), detector_like.metadata_arc(), true);

        let hit = dispatcher
            .locate_plugin("github.com ")
            .await
            .expect("无触发词的自定义匹配应接管");
        assert_eq!(hit.plugin_id, "detector-like");
        assert_eq!(hit.search_term, "github.com ");
        assert_eq!(hit.match_model, Some(InputMatch::Custom));

        // 与关键词插件竞争同一输入：priority 小者（关键词插件，0）胜
        let keyword = Arc::new(TriggerStubPlugin::with_trigger_and_id("path", "kw-path"));
        dispatcher.register_plugin(keyword.clone(), keyword.metadata_arc(), true);
        assert_eq!(
            dispatcher
                .locate_plugin("path x")
                .await
                .expect("应命中")
                .plugin_id,
            "kw-path"
        );
    }

    /// 沉浸式（Panel）形态插件不参与查询路由——面板形态仅经热键/候选项唤醒。
    #[tokio::test]
    async fn panel_mode_plugin_does_not_route() {
        let dispatcher = SessionDispatcher::new(Arc::new(PluginRegistry::new()));
        let plugin = Arc::new(TriggerStubPlugin::with_panel_trigger("paneltrig"));
        dispatcher.register_plugin(plugin.clone(), plugin.metadata_arc(), true);

        assert!(dispatcher.locate_plugin("paneltrig x").await.is_none());
    }

    /// 热键唤醒测试用面板桩 —— query 返回 CustomPanel（形态由构造参数决定）。
    struct PanelStubPlugin {
        metadata: PluginMetadata,
        core: ComponentCore,
        keep_search_bar: bool,
    }

    impl PanelStubPlugin {
        /// 该桩的插件级元数据副本（注册中心条目数据源）。
        fn metadata_arc(&self) -> Arc<PluginMetadata> {
            Arc::new(self.metadata.clone())
        }

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
                    trigger_keywords: vec!["test.panel".to_string()],
                    supported_os: Vec::new(),
                    priority: 0,
                    kind: PluginKind::Builtin,
                    hotkey: Some("Ctrl+E".to_string()),
                    icon: None,
                    mode: PluginMode::Panel,
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
        let plugin = Arc::new(PanelStubPlugin::new());
        dispatcher.register_plugin(plugin.clone(), plugin.metadata_arc(), true);

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

        assert_eq!(active_plugin_id(&dispatcher).as_deref(), Some("test.panel"));
        assert_eq!(dispatcher.current_view_str(), "plugin_immersive");

        let events = events.lock();
        let event = events.last().expect("应推送会话事件");
        assert_eq!(event_plugin_id(event), Some("test.panel"));
        let interaction = event_interaction(event).expect("插件事件应携带交互契约");
        let _ = &interaction.bindings;
        let content = event_panel_content(event).expect("唤醒推送应携带面板载荷");
        assert_eq!(content.panel_type, "test-panel");
        assert_eq!(content.data, serde_json::json!({ "hello": "world" }));
    }

    /// 禁用插件不可被热键唤醒（后端权威校验，前端热键表残留过期条目时兜底）。
    #[tokio::test]
    async fn wake_plugin_rejects_disabled_plugin() {
        let dispatcher = SessionDispatcher::new(Arc::new(PluginRegistry::new()));
        dispatcher.set_host_api(test_host_api());
        let plugin = Arc::new(PanelStubPlugin::new());
        // 禁用状态注册（enabled=false）→ 不在启用集合
        dispatcher.register_plugin(plugin.clone(), plugin.metadata_arc(), false);

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
        // TriggerStubPlugin 的 query 返回 Empty（非 CustomPanel）；panel 形态才能通过 mode 校验
        let plugin = Arc::new(TriggerStubPlugin::with_panel_trigger("="));
        let plugin_id = plugin.metadata.id.clone();
        dispatcher.register_plugin(plugin.clone(), plugin.metadata_arc(), true);

        let err = dispatcher.wake_plugin(&plugin_id).await.unwrap_err();
        assert!(
            err.to_string().contains("CustomPanel"),
            "应拒绝非 CustomPanel 响应: {}",
            err
        );
    }

    /// 热键唤醒仅支持 panel 形态：行内插件即使声明热键也被 mode 校验拒绝。
    #[tokio::test]
    async fn wake_plugin_rejects_inline_mode() {
        let dispatcher = SessionDispatcher::new(Arc::new(PluginRegistry::new()));
        dispatcher.set_host_api(test_host_api());
        // inline 形态 + 声明热键 → mode 校验拒绝（不进入查询）
        let plugin = Arc::new(TriggerStubPlugin::with_inline_hotkey_trigger("="));
        let plugin_id = plugin.metadata.id.clone();
        dispatcher.register_plugin(plugin.clone(), plugin.metadata_arc(), true);

        let err = dispatcher.wake_plugin(&plugin_id).await.unwrap_err();
        assert!(
            err.to_string().contains("行内形态"),
            "应拒绝 inline 形态热键唤醒: {}",
            err
        );
    }

    /// 候选项唤醒走统一 executor 管道：插件候选选中（action "open"）经
    /// PluginWakeExecutor 解析并执行，最终唤醒面板会话。
    #[tokio::test]
    async fn plugin_candidate_confirm_wakes_via_executor_pipeline() {
        let dispatcher = Arc::new(SessionDispatcher::new(Arc::new(PluginRegistry::new())));
        dispatcher.set_host_api(test_host_api());
        dispatcher.register_executor(Arc::new(
            super::super::plugin_wake_executor::PluginWakeExecutor::new(Arc::downgrade(
                &dispatcher,
            )),
        ));

        let plugin = Arc::new(PanelStubPlugin::new());
        let plugin_id = plugin.metadata.id.clone();
        dispatcher.register_plugin(plugin.clone(), plugin.metadata_arc(), true);

        // 构造插件候选并刷新进缓存
        {
            let candidates = dispatcher.build_plugin_candidates();
            assert_eq!(candidates.len(), 1, "应生成 1 个插件候选项");
            assert_eq!(
                candidates[0].target,
                ExecutionTarget::Plugin(plugin_id.clone())
            );
            assert!(
                candidates[0].keywords.contains(&plugin_id),
                "候选关键字应包含触发词"
            );
            assert!(
                candidates[0].keywords.iter().any(|k| k.contains("面板桩")),
                "候选关键字应包含插件名称"
            );
            assert!(
                matches!(candidates[0].icon, IconRequest::Data(_)),
                "候选图标应为 Data 直通（插件元数据 data URL）"
            );
        }
        dispatcher.refresh_candidates().await;

        // 查找插件候选 id（缓存重新分配）
        let candidate_id = dispatcher
            .cached_candidates
            .read()
            .get_candidates()
            .iter()
            .find(|c| matches!(c.target, ExecutionTarget::Plugin(_)))
            .expect("缓存中应存在插件候选")
            .id;

        // 统一确认路径：resolve → execute → wake_plugin
        dispatcher
            .execute_candidate(candidate_id, 0, "open", "", &[])
            .await
            .expect("插件候选确认应成功");

        assert_eq!(active_plugin_id(&dispatcher).as_deref(), Some("test.panel"));
        assert_eq!(dispatcher.current_view_str(), "plugin_immersive");
    }

    /// 热键唤醒 = 全页面接管：keep_search_bar=true（行内面板）属契约违约，
    /// debug 构建用 debug_assert 强制 panic 暴露（测试构建即 debug_assertions 开启，
    /// 故此处应 panic）；release 构建降级为 PluginPanel 正常唤醒（该分支在
    /// debug_assertions 下不可达，release 路径不在此单测范围内）。
    #[tokio::test]
    #[should_panic(expected = "keep_search_bar=true")]
    async fn wake_plugin_panics_on_keep_search_bar() {
        let dispatcher = SessionDispatcher::new(Arc::new(PluginRegistry::new()));
        dispatcher.set_host_api(test_host_api());
        let plugin = Arc::new(PanelStubPlugin::with_keep_search_bar(true));
        dispatcher.register_plugin(plugin.clone(), plugin.metadata_arc(), true);

        // 应 panic（断言消息含 keep_search_bar=true），不返回
        let _ = dispatcher.wake_plugin("test.panel").await;
    }

    /// 路由测试桩 —— 记录每次 query 调用的（通道, search_term, raw_query, confirm），
    /// 可选延迟配合提交门控验证。
    /// 记录的查询调用：(通道, search_term, raw_query, confirm)。
    type RecordedCall = (QueryChannel, String, String, bool);

    struct RecordingStubPlugin {
        metadata: PluginMetadata,
        core: ComponentCore,
        calls: Arc<Mutex<Vec<RecordedCall>>>,
        delay: Option<std::time::Duration>,
    }

    impl RecordingStubPlugin {
        /// 该桩的插件级元数据副本（注册中心条目数据源）。
        fn metadata_arc(&self) -> Arc<PluginMetadata> {
            Arc::new(self.metadata.clone())
        }

        fn new(trigger: &str, calls: Arc<Mutex<Vec<RecordedCall>>>) -> Self {
            let id = format!("test.{}", trigger);
            Self {
                metadata: PluginMetadata {
                    id: id.clone(),
                    name: format!("recording-{}", trigger),
                    version: "0.1.0".to_string(),
                    description: "路由测试桩".to_string(),
                    author: "test".to_string(),
                    trigger_keywords: vec![trigger.to_string()],
                    supported_os: Vec::new(),
                    priority: 0,
                    kind: PluginKind::Builtin,
                    hotkey: None,
                    icon: None,
                    mode: PluginMode::Inline,
                },
                core: ComponentCore::new(
                    id,
                    "路由测试桩".to_string(),
                    "查询入口验证".to_string(),
                    ComponentType::Plugin,
                    0,
                ),
                calls,
                delay: None,
            }
        }

        /// 指定查询延迟：模拟慢查询，配合提交门控测试。
        fn with_delay(mut self, delay: std::time::Duration) -> Self {
            self.delay = Some(delay);
            self
        }
    }

    impl Configurable for RecordingStubPlugin {
        fn core(&self) -> &ComponentCore {
            &self.core
        }

        fn setting_schema(&self) -> Vec<SettingDefinition> {
            Vec::new()
        }
    }

    #[async_trait]
    impl Plugin for RecordingStubPlugin {
        /// 面板按键契约：Escape → 返回。
        /// 会话投递断链时前端拿不到该声明（面板退出等按键全部失效），供投递不变式测试断言。
        fn interaction_policy(&self) -> PanelInteraction {
            PanelInteraction {
                bindings: vec![PanelKeyBinding {
                    key: "Escape".to_string(),
                    action: PanelKeyAction::GoBack,
                }],
                ..Default::default()
            }
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
            ctx: &PluginContext,
            query: &Query,
        ) -> Result<QueryResponse, PluginError> {
            if let Some(delay) = self.delay {
                tokio::time::sleep(delay).await;
            }
            self.calls.lock().push((
                ctx.query_channel,
                query.search_term.clone(),
                query.raw_query.clone(),
                query.confirm,
            ));
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

    /// 面板直调（explicit_plugin_id + Panel 通道）：search_term 小写化、confirm 固定
    /// false、不改写会话（只读辅助路径）。
    #[tokio::test]
    async fn route_query_panel_is_readonly_direct_call() {
        let dispatcher = SessionDispatcher::new(Arc::new(PluginRegistry::new()));
        let calls = Arc::new(Mutex::new(Vec::new()));
        let plugin = Arc::new(RecordingStubPlugin::new("=", calls.clone()));
        dispatcher.register_plugin(plugin.clone(), plugin.metadata_arc(), true);

        // 调用方 confirm=true：面板直调必须强制为 false
        let query = Query {
            id: "trace".to_string(),
            raw_query: "HeLLo WoRLd".to_string(),
            search_term: String::new(),
            confirm: true,
        };
        let routed = dispatcher
            .route_query_panel("trace-1", &query, "test.=")
            .await
            .expect("面板直调应成功");

        let (channel, search_term, raw_query, confirm) = calls.lock()[0].clone();
        assert_eq!(channel, QueryChannel::Panel, "应透传 Panel 通道");
        assert_eq!(search_term, "hello world", "search_term 应为输入小写化");
        assert_eq!(raw_query, "HeLLo WoRLd", "raw_query 应保持原文");
        assert!(!confirm, "面板直调 confirm 必须固定 false");
        assert_eq!(
            routed.plugin_id.as_deref(),
            Some("test.="),
            "响应应回填插件 id"
        );
        assert_eq!(
            dispatcher.current_view_str(),
            "none",
            "Panel 通道不改写会话"
        );
    }

    /// 触发词路由（Ui 通道）：search_term 剥离触发词、透传 confirm、命中后写入会话投影。
    #[tokio::test]
    async fn route_query_ui_routes_trigger_and_enters_session() {
        let dispatcher = SessionDispatcher::new(Arc::new(PluginRegistry::new()));
        let calls = Arc::new(Mutex::new(Vec::new()));
        let plugin = Arc::new(RecordingStubPlugin::new("=", calls.clone()));
        dispatcher.register_plugin(plugin.clone(), plugin.metadata_arc(), true);

        let query = Query {
            id: "trace".to_string(),
            raw_query: "= 1+1".to_string(),
            search_term: "= 1+1".to_string(),
            confirm: true,
        };
        let routed = dispatcher
            .route_query_ui("trace-1", &query)
            .await
            .expect("触发词路由应成功");

        let (channel, search_term, raw_query, confirm) = calls.lock()[0].clone();
        assert_eq!(channel, QueryChannel::Ui);
        assert_eq!(search_term, "1+1", "search_term 应为剥离触发词后的内容");
        assert_eq!(raw_query, "= 1+1", "raw_query 保持原文");
        assert!(confirm, "触发词路由应透传 confirm");
        assert_eq!(routed.plugin_id.as_deref(), Some("test.="));
        assert_eq!(
            dispatcher.current_view_str(),
            "plugin_panel",
            "UI 通道插件命中应写入会话投影"
        );
    }

    /// 无触发词命中且未显式指定插件 → 默认搜索（管道未初始化返回空结果，不回填插件）。
    #[tokio::test]
    async fn route_query_ui_without_plugin_falls_back_to_search() {
        let dispatcher = SessionDispatcher::new(Arc::new(PluginRegistry::new()));
        let query = Query {
            id: "trace".to_string(),
            raw_query: "hello".to_string(),
            search_term: "hello".to_string(),
            confirm: false,
        };
        let routed = dispatcher
            .route_query_ui("trace-1", &query)
            .await
            .expect("默认搜索应成功");
        assert!(matches!(routed.response, QueryResponse::Empty));
        assert_eq!(routed.plugin_id, None);
    }

    /// UI 入口的行内参数投影：响应为 InlineParam 时写入行内参数子状态与展示形态
    /// （`apply_session_projection` 从响应推导的唯一语义派生分支）。
    #[tokio::test]
    async fn route_query_ui_inline_param_writes_inline_projection() {
        let dispatcher = SessionDispatcher::new(Arc::new(PluginRegistry::new()));
        // 无引擎管道：候选零分透传，行内参数检测仍按缓存候选进行。
        dispatcher.set_search_pipeline(SearchPipeline::without_engine(Vec::new(), 10));
        let mut cache = CachedCandidateData::new();
        cache.add_candidate(SearchCandidate {
            id: 0,
            name: "回声".to_string(),
            icon: IconRequest::Path(String::new()),
            target: ExecutionTarget::Command("echo {}".to_string()),
            keywords: vec!["echo".to_string()],
            bias: 0.0,
            trigger_keywords: vec!["echo".to_string()],
        });
        dispatcher.set_cached_candidates(cache);

        let query = Query {
            id: "trace".to_string(),
            // 尾随空格 + 触发词精确匹配 → 行内参数入口
            raw_query: "echo ".to_string(),
            search_term: "echo ".to_string(),
            confirm: false,
        };
        let routed = dispatcher
            .route_query_ui("trace-1", &query)
            .await
            .expect("行内参数路由应成功");

        let (inline_candidate_id, trigger_keyword, user_arg_count) = match routed.response {
            QueryResponse::InlineParam {
                candidate_id,
                trigger_keyword,
                user_arg_count,
            } => (candidate_id, trigger_keyword, user_arg_count),
            other => panic!("响应应为 InlineParam: {:?}", other),
        };
        assert_eq!(trigger_keyword, "echo");
        assert_eq!(user_arg_count, 1);
        assert_eq!(
            dispatcher.current_view_str(),
            "inline_param",
            "InlineParam 响应应写入行内参数展示形态"
        );
        assert!(
            matches!(
                *dispatcher.search_state.read(),
                SearchSubState::InlineParam { candidate_id } if candidate_id == inline_candidate_id
            ),
            "行内参数子状态应锁定响应中的候选项"
        );
    }

    /// 显式直调不存在的插件 → InvalidState（错误消息带入口上下文）。
    #[tokio::test]
    async fn route_query_panel_explicit_unknown_plugin_errors() {
        let dispatcher = SessionDispatcher::new(Arc::new(PluginRegistry::new()));
        let query = Query {
            id: "trace".to_string(),
            raw_query: "x".to_string(),
            search_term: "x".to_string(),
            confirm: false,
        };
        let err = dispatcher
            .route_query_panel("trace-1", &query, "ghost")
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("面板查询"),
            "错误消息应带入口上下文: {}",
            err
        );
    }

    /// 提交门控（仅 UI 入口有版本域）：慢查询执行期间更新的查询进入 → 旧查询过期丢弃
    /// （空响应 + 仍回填归属），且**不得改写会话投影**（快查询的投影必须保留）。
    /// 只读入口（CLI/面板）不分配版本号，其查询结果不作废。
    #[tokio::test]
    async fn route_query_ui_stale_query_discarded_without_overwriting_projection() {
        let dispatcher = Arc::new(SessionDispatcher::new(Arc::new(PluginRegistry::new())));
        dispatcher.set_search_pipeline(SearchPipeline::without_engine(Vec::new(), 10));
        let calls = Arc::new(Mutex::new(Vec::new()));
        let plugin = Arc::new(
            RecordingStubPlugin::new("=", calls.clone())
                .with_delay(std::time::Duration::from_millis(100)),
        );
        dispatcher.register_plugin(plugin.clone(), plugin.metadata_arc(), true);

        // 慢查询：触发词命中 → 本应进入 test.= 插件面板会话
        let slow_query = Query {
            id: "trace".to_string(),
            raw_query: "= 1+1".to_string(),
            search_term: "= 1+1".to_string(),
            confirm: false,
        };
        let spawned = dispatcher.clone();
        let slow = tauri::async_runtime::spawn(async move {
            spawned.route_query_ui("trace-1", &slow_query).await
        });
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;

        // 快查询：无触发词命中 → 默认搜索投影（搜索形态）
        let fast_query = Query {
            id: "trace".to_string(),
            raw_query: "hello".to_string(),
            search_term: "hello".to_string(),
            confirm: false,
        };
        let fast = dispatcher
            .route_query_ui("trace-2", &fast_query)
            .await
            .expect("新查询应成功");
        assert!(matches!(fast.response, QueryResponse::List { .. }));

        let slow = slow
            .await
            .expect("慢查询任务不应 panic")
            .expect("慢查询应返回路由结果");
        assert!(
            matches!(slow.response, QueryResponse::Empty),
            "慢查询已过期应丢弃结果"
        );
        assert_eq!(
            slow.plugin_id.as_deref(),
            Some("test.="),
            "过期丢弃仍回填插件 id"
        );
        assert_eq!(
            active_plugin_id(&dispatcher),
            None,
            "过期查询不得把会话投影改写为插件归属"
        );
        assert_eq!(
            dispatcher.current_view_str(),
            "search",
            "投影应保留快查询写入的搜索形态"
        );
    }

    // ==================== 会话投递不变式 ====================

    /// 测试用窗口行为配置桩：仅承载会话层读取的 `is_show_home_on_empty_query`。
    ///
    /// 仅限本测试模块使用（以桩替代内置配置组件，测试不引用 builtin_plugin 实现）。
    struct WindowBehaviorStub {
        /// 组件身份元数据（component_id 必须为 "window-behavior-config"）。
        core: ComponentCore,
        /// 对外暴露的配置值（会话层经 get_component_setting 读取单字段）。
        settings: serde_json::Value,
    }

    impl WindowBehaviorStub {
        /// 构造常驻结果框开关为 `home_enabled` 的配置桩。
        fn new(home_enabled: bool) -> Self {
            Self {
                core: ComponentCore::new(
                    "window-behavior-config".to_string(),
                    "窗口行为配置桩".to_string(),
                    String::new(),
                    ComponentType::Core,
                    0,
                ),
                settings: serde_json::json!({ "is_show_home_on_empty_query": home_enabled }),
            }
        }
    }

    impl Configurable for WindowBehaviorStub {
        fn core(&self) -> &ComponentCore {
            &self.core
        }

        /// 声明被读取的单字段（注册时组件默认配置需通过 schema 校验）。
        fn setting_schema(&self) -> Vec<SettingDefinition> {
            vec![SchemaBuilder::boolean(
                "is_show_home_on_empty_query",
                "常驻结果框",
                "空查询显示常用候选项",
            )
            .build()]
        }

        fn get_settings(&self) -> serde_json::Value {
            self.settings.clone()
        }
    }

    /// 注入承载常驻结果框开关的 ConfigManager（未注入时会话层按"开启"处理）。
    async fn set_home_setting(dispatcher: &SessionDispatcher, home_enabled: bool) {
        let cm = Arc::new(ConfigManager::new(
            std::env::temp_dir().join("zl-home-setting-test"),
        ));
        cm.register(Arc::new(WindowBehaviorStub::new(home_enabled)))
            .await;
        dispatcher.set_config_manager(cm);
    }

    /// 触发词 + 空格分隔内容的插件查询（路由命中 "=" 桩插件）。
    fn plugin_query() -> Query {
        Query {
            id: "trace".to_string(),
            raw_query: "= 1+1".to_string(),
            search_term: "= 1+1".to_string(),
            confirm: false,
        }
    }

    /// 空查询（清空输入 / 退出面板路径发出的查询）。
    fn empty_query() -> Query {
        Query {
            id: "trace".to_string(),
            raw_query: String::new(),
            search_term: String::new(),
            confirm: false,
        }
    }

    /// 捕获会话事件的 emitter 与事件容器（测试共用）。
    fn capture_session_events(
        dispatcher: &SessionDispatcher,
    ) -> Arc<Mutex<Vec<SessionStateEvent>>> {
        let events = Arc::new(Mutex::new(Vec::new()));
        let capture = events.clone();
        dispatcher.set_session_emitter(Arc::new(move |event| capture.lock().push(event)));
        events
    }

    /// 事件形态词（宿主/插件两种形态统一到 snake_case 词表）。
    fn event_view_str(event: &SessionStateEvent) -> &'static str {
        match event {
            SessionStateEvent::Host { view, .. } => view.as_str(),
            SessionStateEvent::Plugin { identity, .. } => identity.view.as_str(),
        }
    }

    /// 事件的插件归属 id（宿主事件为 None）。
    fn event_plugin_id(event: &SessionStateEvent) -> Option<&str> {
        match event {
            SessionStateEvent::Host { .. } => None,
            SessionStateEvent::Plugin { identity, .. } => Some(identity.plugin_id.as_str()),
        }
    }

    /// 事件的交互契约（宿主事件无此事实，为 None）。
    fn event_interaction(event: &SessionStateEvent) -> Option<&PanelInteraction> {
        match event {
            SessionStateEvent::Host { .. } => None,
            SessionStateEvent::Plugin { interaction, .. } => Some(interaction),
        }
    }

    /// 事件携带的面板渲染载荷（宿主事件与关键词查询投递均为 None）。
    fn event_panel_content(event: &SessionStateEvent) -> Option<&PluginPanelContent> {
        match event {
            SessionStateEvent::Host { .. } => None,
            SessionStateEvent::Plugin { panel_content, .. } => panel_content.as_deref(),
        }
    }

    /// 事件的关键词镜像触发词（宿主事件与非关键词模型为空）。
    fn event_trigger_keywords(event: &SessionStateEvent) -> &[String] {
        match event {
            SessionStateEvent::Host { .. } => &[],
            SessionStateEvent::Plugin { identity, .. } => match &identity.input_match {
                Some(InputMatch::Keywords { trigger_keywords }) => trigger_keywords,
                _ => &[],
            },
        }
    }

    /// 活动会话的插件归属 id（宿主会话为 None）。
    fn active_plugin_id(dispatcher: &SessionDispatcher) -> Option<String> {
        match dispatcher.current_session().owner {
            SessionOwner::Plugin(identity) => Some(identity.plugin_id),
            SessionOwner::Host(_) => None,
        }
    }

    /// 投递不变式：UI 查询每次命中同一插件都必须投递会话投影。
    ///
    /// 前端渲染插件面板所需的归属/交互契约/触发词只随 session-state 下发，而前端可在
    /// 不发任何 IPC 的情况下本地退出面板（后端无从观测）→ 后端不得按"投影未变"裁剪投递。
    /// 回归：曾按投影变化裁剪，导致同面板重入丢失交互契约（Escape 等按键失效）。
    #[tokio::test]
    async fn ui_plugin_hit_delivers_session_on_every_hit() {
        let dispatcher = SessionDispatcher::new(Arc::new(PluginRegistry::new()));
        dispatcher.set_host_api(test_host_api());
        let events = capture_session_events(&dispatcher);
        let plugin = Arc::new(RecordingStubPlugin::new(
            "=",
            Arc::new(Mutex::new(Vec::new())),
        ));
        dispatcher.register_plugin(plugin.clone(), plugin.metadata_arc(), true);

        dispatcher
            .route_query_ui("trace-1", &plugin_query())
            .await
            .expect("首次命中应成功");
        dispatcher
            .route_query_ui("trace-2", &plugin_query())
            .await
            .expect("再次命中应成功");

        let events = events.lock();
        assert_eq!(events.len(), 2, "每次命中都必须投递会话投影");
        for event in events.iter() {
            assert_eq!(
                event_plugin_id(event),
                Some("test.="),
                "投递须携带会话归属（面板动作回传 pluginId 依赖）"
            );
            let interaction = event_interaction(event).expect("投递须携带交互契约");
            assert!(
                interaction.bindings.iter().any(|b| b.key == "Escape"),
                "投递须携带按键声明（否则面板无法退出）"
            );
            assert!(
                event_trigger_keywords(event).contains(&"=".to_string()),
                "投递须携带触发词（前端退出判定镜像参数）"
            );
        }
        assert_eq!(
            event_generation(&events[0]),
            event_generation(&events[1]),
            "投影未变：重复投递不递增代际"
        );
    }

    /// 事件代际（两类会话共有字段）。
    fn event_generation(event: &SessionStateEvent) -> u64 {
        match event {
            SessionStateEvent::Host { generation, .. } => *generation,
            SessionStateEvent::Plugin { generation, .. } => *generation,
        }
    }

    /// 变更通知语义：默认搜索查询只在投影变化时推送（载荷随 bridge_query 响应下发）。
    /// 与插件命中路径的无条件投递构成有意的不对称，防止有人统一改成"每次必推"。
    #[tokio::test]
    async fn default_search_query_notifies_only_on_projection_change() {
        let dispatcher = SessionDispatcher::new(Arc::new(PluginRegistry::new()));
        dispatcher.set_search_pipeline(SearchPipeline::without_engine(Vec::new(), 10));
        let events = capture_session_events(&dispatcher);

        let query = Query {
            id: "trace".to_string(),
            raw_query: "hello".to_string(),
            search_term: "hello".to_string(),
            confirm: false,
        };
        for trace in ["trace-1", "trace-2"] {
            dispatcher
                .route_query_ui(trace, &query)
                .await
                .expect("默认搜索应成功");
        }

        assert_eq!(events.lock().len(), 1, "投影未变的搜索查询不重复推送");
    }

    /// 只读入口（面板/CLI）命中插件不写投影、不投递（结构上无会话副作用）。
    #[tokio::test]
    async fn readonly_query_entries_do_not_deliver_session() {
        let dispatcher = SessionDispatcher::new(Arc::new(PluginRegistry::new()));
        dispatcher.set_host_api(test_host_api());
        let events = capture_session_events(&dispatcher);
        let plugin = Arc::new(RecordingStubPlugin::new(
            "=",
            Arc::new(Mutex::new(Vec::new())),
        ));
        dispatcher.register_plugin(plugin.clone(), plugin.metadata_arc(), true);

        dispatcher
            .route_query_panel("trace-1", &plugin_query(), "test.=")
            .await
            .expect("面板直调应成功");
        dispatcher
            .route_query_cli("trace-2", &plugin_query())
            .await
            .expect("CLI 查询应成功");

        assert!(events.lock().is_empty(), "只读入口不得投递会话事件");
        assert_eq!(
            dispatcher.current_view_str(),
            "none",
            "只读入口不得改写会话投影"
        );
    }

    /// 空查询 + 关闭常驻结果框 → 结束会话（后端裁决，前端无需显式声明会话结束）。
    #[tokio::test]
    async fn empty_ui_query_ends_session_when_home_disabled() {
        let dispatcher = SessionDispatcher::new(Arc::new(PluginRegistry::new()));
        dispatcher.set_host_api(test_host_api());
        set_home_setting(&dispatcher, false).await;
        let events = capture_session_events(&dispatcher);
        let plugin = Arc::new(RecordingStubPlugin::new(
            "=",
            Arc::new(Mutex::new(Vec::new())),
        ));
        dispatcher.register_plugin(plugin.clone(), plugin.metadata_arc(), true);

        dispatcher
            .route_query_ui("trace-1", &plugin_query())
            .await
            .expect("插件命中应成功");
        assert_eq!(
            dispatcher.current_view_str(),
            "plugin_panel",
            "前置：已进入插件会话"
        );

        let routed = dispatcher
            .route_query_ui("trace-2", &empty_query())
            .await
            .expect("空查询应成功");

        assert!(
            matches!(routed.response, QueryResponse::Empty),
            "空查询应返回空响应"
        );
        assert_eq!(active_plugin_id(&dispatcher), None, "空查询应结束插件会话");
        assert_eq!(
            dispatcher.current_view_str(),
            "none",
            "空查询应复位会话投影"
        );
        assert_eq!(
            events
                .lock()
                .last()
                .map(event_view_str)
                .expect("应推送会话结束投影"),
            "none",
            "会话结束经 session-state 投递（前端据此复位本地状态）"
        );
    }

    /// 空查询 + 开启常驻结果框 → 走主页搜索（投影为搜索形态，不结束会话）。
    #[tokio::test]
    async fn empty_ui_query_loads_home_when_enabled() {
        let dispatcher = SessionDispatcher::new(Arc::new(PluginRegistry::new()));
        dispatcher.set_host_api(test_host_api());
        dispatcher.set_search_pipeline(SearchPipeline::without_engine(Vec::new(), 10));
        set_home_setting(&dispatcher, true).await;
        let plugin = Arc::new(RecordingStubPlugin::new(
            "=",
            Arc::new(Mutex::new(Vec::new())),
        ));
        dispatcher.register_plugin(plugin.clone(), plugin.metadata_arc(), true);

        dispatcher
            .route_query_ui("trace-1", &plugin_query())
            .await
            .expect("插件命中应成功");

        dispatcher
            .route_query_ui("trace-2", &empty_query())
            .await
            .expect("空查询应成功");

        assert_eq!(
            dispatcher.current_view_str(),
            "search",
            "常驻结果框开启时空查询进入搜索形态（主页）"
        );
        assert_eq!(active_plugin_id(&dispatcher), None);
    }
}
