use crate::config::Configurable;
use crate::plugin::cached_candidate::CachedCandidateData;
use crate::services::icon_request::IconRequest;
use crate::services::parameter::types::ParameterSnapshot;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

pub type CandidateId = u64;

/// 执行目标类型枚举，用于 ActionExecutor 注册和查找
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum TargetType {
    Path,
    App,
    File,
    Url,
    Command,
    BuiltinCommand,
}

impl TargetType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TargetType::Path => "Path",
            TargetType::App => "App",
            TargetType::File => "File",
            TargetType::Url => "Url",
            TargetType::Command => "Command",
            TargetType::BuiltinCommand => "BuiltinCommand",
        }
    }
}

/// 执行目标
#[derive(Debug, Clone, Serialize, Deserialize, Eq, Hash, PartialEq)]
pub enum ExecutionTarget {
    #[serde(rename = "path")]
    Path(String),
    #[serde(rename = "app")]
    App(String),
    #[serde(rename = "file")]
    File(String),
    #[serde(rename = "url")]
    Url(String),
    #[serde(rename = "command")]
    Command(String),
    #[serde(rename = "builtinCommand")]
    BuiltinCommand(String),
}

impl ExecutionTarget {
    pub fn target_type(&self) -> TargetType {
        match self {
            ExecutionTarget::Path(_) => TargetType::Path,
            ExecutionTarget::App(_) => TargetType::App,
            ExecutionTarget::File(_) => TargetType::File,
            ExecutionTarget::Url(_) => TargetType::Url,
            ExecutionTarget::Command(_) => TargetType::Command,
            ExecutionTarget::BuiltinCommand(_) => TargetType::BuiltinCommand,
        }
    }

    pub fn payload(&self) -> &str {
        match self {
            ExecutionTarget::Path(s) => s,
            ExecutionTarget::App(s) => s,
            ExecutionTarget::File(s) => s,
            ExecutionTarget::Url(s) => s,
            ExecutionTarget::Command(s) => s,
            ExecutionTarget::BuiltinCommand(s) => s,
        }
    }
}

/// 执行上下文
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub target: ExecutionTarget,
    pub display_name: String,
    /// 用户输入的参数列表
    pub user_args: Vec<String>,
    /// 系统参数快照（不透明句柄）
    pub parameter_snapshot: ParameterSnapshot,
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self {
            target: ExecutionTarget::Path(String::new()),
            display_name: String::new(),
            user_args: Vec::new(),
            parameter_snapshot: ParameterSnapshot::empty(),
        }
    }
}

/// 执行错误
#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    #[error("Execution failed: {0}")]
    Failed(String),

    #[error("Executor not found for target type: {0:?}")]
    NotFound(TargetType),

    #[error("Unsupported action: {0:?}:{1}")]
    UnsupportedAction(TargetType, String),

    /// 窗口唤醒失败，携带回退目标
    /// Executor 声明回退策略，Registry 负责执行回退
    #[error("Window activation failed, fallback to: {fallback_action}")]
    ActivationFailed { fallback_action: String },
}

/// 注册错误
#[derive(Debug, thiserror::Error)]
pub enum RegistrationError {
    #[error("Action '{action_id}' for {target_type:?} is already registered")]
    ActionConflict {
        target_type: TargetType,
        action_id: String,
    },
}

// 这个是一个搜索候选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchCandidate {
    // 候选项的唯一标识符
    #[serde(rename = "id")]
    pub id: CandidateId,
    // 表示用于显示在搜索结果中的名称
    #[serde(rename = "name")]
    pub name: String,
    // 表示用于显示在搜索结果中的图标
    #[serde(rename = "icon")]
    pub icon: IconRequest,
    // 执行目标，替代原 launch_method
    #[serde(rename = "target")]
    pub target: ExecutionTarget,
    // 表示该候选项的关键词，即怎么可以确认用户想要启动这个候选项
    #[serde(rename = "keywords")]
    pub keywords: Vec<String>,
    // 固定的权重偏移，用于在计算分数时考虑该候选项的固定权重。由每个数据源来控制各自的权重
    #[serde(rename = "bias")]
    pub bias: f64,
    /// 触发关键词列表，用于行内模式的精确匹配
    #[serde(rename = "triggerKeywords")]
    pub trigger_keywords: Vec<String>,
}

/// 分数明细的计入方式 —— 标识该项是加权加分还是乘法系数。
///
/// 跨 IPC 序列化，由引擎/增强器构造，前端按 kind 渲染明细形态：
/// 加法项显示 `score × weight = 乘积`，乘法项显示 `× 系数`，
/// 避免把乘法系数误读为加分项导致总分无法核对。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ScoreDetailKind {
    /// 加权加分项：该项的 score × weight 计入总分（默认）。
    #[default]
    #[serde(rename = "add")]
    Add,
    /// 乘法系数项：该项的 score 乘到当前累计分数上（如长度比率、溢出惩罚、抑制因子）。
    #[serde(rename = "multiply")]
    Multiply,
}

// 这个是一个搜索候选项的详细分数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreDetail {
    // 基础分
    #[serde(rename = "score")]
    pub score: f64,
    // 当前权重分
    #[serde(rename = "weight")]
    pub weight: f64,
    // 这个是什么分，以及这个分的来源
    #[serde(rename = "description")]
    pub description: String,
    // 该项的计入方式：add = 加权加分，multiply = 乘法系数
    #[serde(rename = "kind", default)]
    pub kind: ScoreDetailKind,
}

// 这个是一个搜索候选项的分数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredCandidate {
    // 表示该候选项的分数
    #[serde(rename = "candidateId")]
    pub candidate_id: CandidateId,
    // 表示该候选项的分数
    #[serde(rename = "score")]
    pub score: f64,
    //表示该候选项得来的详细的分数：加法项按 sum(score × weight) 计入，
    //乘法项按系数乘入（引擎先乘系数再加加法项，增强器仅产出加法项）
    #[serde(rename = "detailedScore")]
    pub detailed_score: Vec<ScoreDetail>,
}

// 表示一个数据源
#[async_trait]
pub trait DataSource: Configurable {
    async fn fetch_candidates(&self) -> CachedCandidateData;
}

// 表示对搜索的候选项的搜索关键字做优化的组件，通常是对搜索关键字进行扩展或者优化，以提高搜索的召回率
pub trait KeywordOptimizer: Configurable {
    // 根据关键词优化出一组新关键词，通常是对关键词进行分词、扩展或转换
    fn optimize(&self, keyword: &str) -> Vec<String>;
    // 是否对所有已累积的关键词进行优化（true），还是只对原始名称优化
    fn uses_context(&self) -> bool {
        false
    }
    // 获得优先级，优先级小的优化器会先被调用，优先级相同的优化器会按照注册的顺序被调用
    fn get_priority(&self) -> i32;
}

/// 根据候选项的完整上下文注入额外关键字。
/// 与 KeywordOptimizer 不同，此方法可以访问候选项的 target、icon 等完整信息，
/// 用于实现"基于候选身份的关键字注入"（如别名）。
pub trait KeywordInjector: Configurable {
    /// 根据候选项的完整上下文注入额外关键字。
    /// 返回注入的关键字列表。
    fn inject_keywords(&self, candidate: &SearchCandidate) -> Vec<String>;
}

// 表示一个搜索引擎，用于计算搜索候选项的分数
// 用于根据搜索候选项的分数进行排序
// 搜索引擎通常计算的是一个候选项与用户输入之间的关系
pub trait SearchEngine: Configurable {
    fn calculate_scores(
        &self,
        candidates: &CachedCandidateData,
        query: &str,
    ) -> Vec<ScoredCandidate>;
}

// 表示一个分数优化器，用于对搜索候选项的分数进行优化
// 用于根据搜索候选项的分数进行排序
// 分数优化器则是计算的是 *所有* 候选项与用户输入之间的关系
pub trait ScoreBooster: Configurable {
    // 记录用户输入了这个查询时，选择的是这个候选项
    fn record(&self, candidate_id: CandidateId, data: &CachedCandidateData, query: &str);
    // 根据用户历史输入的查询与选择的候选项，优化当前查询所得到的所有候选项的分数
    fn boost(&self, candidates: &mut Vec<ScoredCandidate>, data: &CachedCandidateData, query: &str);
}

/// 动作执行器 trait
/// 每个 Executor 可以声明支持多种 TargetType 和多种 Action
/// Executor 继承 Configurable，以支持统一配置管理和发现
#[async_trait]
pub trait ActionExecutor: Configurable {
    /// 返回该 Executor 支持的目标类型集合
    fn supported_target_types(&self) -> Vec<TargetType>;

    /// 返回该 Executor 支持的动作列表
    fn supported_actions(&self) -> Vec<ResultAction> {
        vec![ResultAction {
            id: "execute".to_string(),
            label: "执行".to_string(),
            icon: IconRequest::Path(String::new()),
            is_default: true,
            shortcut_key: String::new(),
        }]
    }

    /// 根据动作 ID 执行对应的操作
    /// 参数：ctx - 执行上下文；action_id - 动作 ID
    /// 返回：执行成功返回 Ok(())，失败返回 ExecutionError
    async fn execute(&self, ctx: &ExecutionContext, action_id: &str) -> Result<(), ExecutionError>;
}

/// 查询来源通道 — 标识查询进入后端的入口，用于通道间隔离。
///
/// 各通道独立维护查询版本计数器，跨通道查询互不使对方过期；
/// 且仅 GUI 通道允许改写会话状态（会话模式、面板交互事件）与
/// 插件侧跨查询共享状态（如剪贴板缓存），CLI/调试查询为只读辅助路径。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum QueryChannel {
    /// 主窗口 GUI 查询（bridge_query）。
    #[default]
    Ui,
    /// 本地 CLI HTTP 查询（/v1/query）。
    Cli,
    /// 调试模拟查询（debug_simulate_query）。
    Debug,
}

/// 查询版本门控：宿主在每次查询入口分配单调递增版本号，
/// 供插件在写入跨查询共享状态（如翻译结果缓存）前判断自身查询
/// 是否已被更新的查询取代。
///
/// 仅宿主进程内使用（内置插件经 PluginContext 注入）；不跨 RPC 传输，
/// 远端插件或直接构造的上下文无门控，此时视为始终为最新。
#[derive(Debug, Clone)]
pub struct QueryRevisionGate {
    /// 当前查询被分配的版本号。
    revision: u64,
    /// 宿主侧「最新已分配版本」计数器（每次查询入口 fetch_add）。
    latest: Arc<AtomicU64>,
}

impl QueryRevisionGate {
    /// 创建门控：revision 为当前查询的版本号，latest 为共享的最新版本计数器。
    pub fn new(revision: u64, latest: Arc<AtomicU64>) -> Self {
        Self { revision, latest }
    }

    /// 当前查询的版本号，供日志与追踪使用。
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// 当前查询是否仍是最新：成立才允许写入跨查询共享状态。
    pub fn is_current(&self) -> bool {
        self.latest.load(Ordering::Relaxed) == self.revision
    }
}

/// 请求级上下文，在宿主与插件之间共享。
/// 服务于插件生命周期/查询/动作调用，并携带日志关联 ID。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginContext {
    // 当前的请求 ID
    pub trace_id: String,
    // 当前的请求 ID
    pub query_id: Option<String>,
    // 处理当前请求的插件 ID
    pub plugin_id: Option<String>,
    /// 查询版本门控（宿主注入；#[serde(skip)] 不跨 RPC 传输，
    /// 远端插件或直接构造的上下文为 None，此时视为始终为最新）。
    #[serde(skip)]
    pub query_revision_gate: Option<QueryRevisionGate>,
    /// 查询来源通道（宿主注入；远端插件经 RPC 反序列化时缺省视为 GUI 通道）。
    #[serde(default)]
    pub query_channel: QueryChannel,
}

impl PluginContext {
    pub fn new(trace_id: &str) -> Self {
        Self {
            trace_id: trace_id.to_string(),
            query_id: None,
            plugin_id: None,
            query_revision_gate: None,
            query_channel: QueryChannel::Ui,
        }
    }

    pub fn with_query(&mut self, query_id: String) {
        self.query_id = Some(query_id);
    }

    pub fn with_plugin_id(&mut self, plugin_id: String) {
        self.plugin_id = Some(plugin_id);
    }

    /// 注入查询版本门控（宿主查询入口调用）。
    pub fn set_query_revision_gate(&mut self, gate: QueryRevisionGate) {
        self.query_revision_gate = Some(gate);
    }

    /// 当前查询是否仍是最新；无门控（远端插件、测试、mock）恒为 true。
    pub fn is_query_current(&self) -> bool {
        self.query_revision_gate
            .as_ref()
            .is_none_or(|g| g.is_current())
    }

    /// 当前查询的版本号；无门控时为 0，仅供日志使用。
    pub fn query_revision(&self) -> u64 {
        self.query_revision_gate
            .as_ref()
            .map_or(0, |g| g.revision())
    }
}

/// 发送给插件查询处理器的标准化查询载荷。
/// 服务于查询分发和插件侧搜索逻辑。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Query {
    /// 本次查询的唯一标识，取自 bridge_query 中生成的 trace_id，用于日志关联和插件上下文。
    pub id: String,
    /// 用户在搜索栏中输入的原始字符串，未经任何处理，用于触发器匹配和日志记录。
    pub raw_query: String,
    /// 派生自 raw_query 的搜索词。普通搜索为全小写形式，插件模式为剥离触发关键词后的剩余部分。
    pub search_term: String,
    /// 是否由用户显式确认（如按 Enter）触发的查询。
    /// 行内插件手动模式（PanelQueryTrigger::OnEnter）用它区分确认查询与预览查询；默认 false。
    #[serde(rename = "confirm", default)]
    pub confirm: bool,
}

/// 插件面板查询触发方式的通用语义。
/// 服务于宿主判断行内插件模式下输入后是否自动发起查询。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PanelQueryTrigger {
    /// 输入后自动触发查询（默认；配合 query_debounce_ms 防抖）。
    #[default]
    #[serde(rename = "onInput")]
    OnInput,
    /// 输入不自动触发查询，由用户按 Enter 手动触发。
    #[serde(rename = "onEnter")]
    OnEnter,
}

/// 插件面板按键绑定 —— 声明式按键契约的最小单元。
/// 服务于宿主解释执行：插件声明按键 → 宿主翻译为动作语义。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PanelKeyBinding {
    /// 按键格式："Enter" | "Ctrl+Enter" | "Escape" | "Tab" | "a"。
    #[serde(rename = "key")]
    pub key: String,
    /// 按键触发的动作。
    #[serde(rename = "action")]
    pub action: PanelKeyAction,
}

/// 面板按键动作 —— 宿主解释执行的动作语义。
/// 服务于插件面板的完整按键权声明（键盘状态机契约）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum PanelKeyAction {
    /// 确认当前面板状态（Enter 标准语义，宿主 confirmQuery 三分支）：
    /// 面板有可执行动作时执行默认动作（如复制结果），否则发起确认查询（翻译/计算/失败重试等）。
    #[serde(rename = "confirm")]
    Confirm,
    /// 执行面板动作：None = 执行面板默认动作（不指定）；Some(id) = 执行指定动作。
    #[serde(rename = "executeAction")]
    ExecuteAction {
        /// 动作 ID：None = 执行面板默认动作；Some = 执行指定动作。
        #[serde(rename = "actionId")]
        action_id: Option<String>,
    },
    /// 返回默认面板。
    #[serde(rename = "goBack")]
    GoBack,
    /// 跳转到同一插件内的子面板。
    #[serde(rename = "gotoPanel")]
    GotoPanel {
        #[serde(rename = "panelId")]
        panel_id: String,
    },
    /// 触发插件自定义动作（经面板动作通道回插件，action 即插件动作 ID）。
    #[serde(rename = "custom")]
    Custom {
        #[serde(rename = "action")]
        action: String,
        #[serde(rename = "args")]
        args: serde_json::Value,
    },
}

/// 插件面板响应携带的通用交互策略。
/// 服务于宿主处理输入查询触发时机，不属于插件持久化配置。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PanelInteraction {
    /// 查询触发方式：onInput 输入自动触发 / onEnter 由用户按 Enter 手动触发。
    #[serde(rename = "queryTrigger", default)]
    pub query_trigger: PanelQueryTrigger,
    /// 后续输入触发查询前的防抖延迟，单位为毫秒（仅 onInput 模式生效）。
    #[serde(rename = "queryDebounceMs", default)]
    pub query_debounce_ms: u64,
    /// 面板按键绑定列表 —— 声明式按键契约（声明即接管：命中绑定由宿主解释执行，
    /// 未声明的键一律交还浏览器/输入框，宿主不做兜底）。
    /// 反序列化缺省为空列表（旧插件未声明时按键全部放行）。
    #[serde(rename = "bindings", default)]
    pub bindings: Vec<PanelKeyBinding>,
}

/// 插件查询响应 —— 一次查询的展示结果契约（跨 IPC 序列化，字段键名与前端
/// `BridgeQueryResponse.mode` 词表对齐）。
///
/// 由 `Plugin::query` / 宿主流程返回，经 SessionDispatcher 路由后包装为
/// `BridgeQueryResponse` 下发前端；四种变体对应前端不同的展示形态。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryResponse {
    /// 候选列表结果 —— 默认搜索与插件均可返回，前端按列表渲染。
    ///
    /// 空列表即空结果（前端映射 mode "search" + 空数组，展示形态层面
    /// 不区分 List/Empty）。
    #[serde(rename = "list")]
    List {
        /// 排序后的候选项列表（含动作、占位符统计、触发关键词等展示元数据）。
        #[serde(rename = "results")]
        results: Vec<ListItem>,
    },
    /// 插件自定义面板 —— 触发式插件接管会话时的渲染结果。
    ///
    /// `keep_search_bar` 决定面板形态：true 为行内面板（保留搜索栏，前端
    /// mode "plugin_panel"），false 为全页面接管（mode "plugin_immersive"）。
    #[serde(rename = "customPanel")]
    CustomPanel {
        /// 面板类型标识，前端按此选择面板组件渲染。
        #[serde(rename = "panelType")]
        panel_type: String,
        /// 面板数据（自由 JSON，面板自行定义结构）。
        #[serde(rename = "data")]
        data: serde_json::Value,
        /// 面板动作列表（供 Enter 执行默认动作 / 面板内动作切换）。
        #[serde(rename = "actions")]
        actions: Vec<ResultAction>,
        /// 是否保留搜索栏（true = 行内面板；false = 全页面接管）。
        #[serde(rename = "keepSearchBar")]
        keep_search_bar: bool,
    },
    /// 空结果 —— 无任何展示内容。
    ///
    /// 前端映射 mode "search" + 空数组（与 `List` 空列表行为一致）。
    /// 注意：插件命中触发词后即使返回 Empty 也是「已处理」，
    /// 不得继续 fallback 到默认搜索。
    #[serde(rename = "empty")]
    Empty,
    /// 行内参数模式：后端检测到触发关键词+空格后自动进入。
    /// 前端据此清空搜索栏并展示参数输入 UI。
    #[serde(rename = "inlineParam")]
    InlineParam {
        /// 目标候选项 ID（确认时回传执行）。
        #[serde(rename = "candidateId")]
        candidate_id: CandidateId,
        /// 命中的触发关键词（前端展示 + 退出判定镜像使用）。
        #[serde(rename = "triggerKeyword")]
        trigger_keyword: String,
        /// 该候选项要求的用户参数个数（前端据此校验输入完整性）。
        #[serde(rename = "userArgCount")]
        user_arg_count: usize,
    },
}

/// 插件返回给宿主的搜索结果项。
/// 服务于结果聚合、排序与 UI 渲染。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListItem {
    // 这个是候选项的唯一标识符
    #[serde(rename = "id")]
    pub id: CandidateId,
    #[serde(rename = "title")]
    pub title: String,
    #[serde(rename = "subtitle")]
    pub subtitle: String,
    #[serde(rename = "icon")]
    pub icon: IconRequest,
    #[serde(rename = "score")]
    pub score: f64,
    // 一个动作列表中只可以有一个默认动作，默认动作会在用户直接按下回车时被触发（由程序员保证）
    #[serde(rename = "actions")]
    pub actions: Vec<ResultAction>,
    /// 目标类型字符串，供前端 ResultItemProvider/ActionInjector 匹配使用
    #[serde(rename = "targetType")]
    pub target_type: String,
    /// 用户参数 {} 的数量
    #[serde(rename = "userArgCount")]
    pub user_arg_count: usize,
    /// 是否包含系统参数（{clip}, {hwnd}, {selection}）
    #[serde(rename = "hasSystemParams")]
    pub has_system_params: bool,
    /// 触发关键词列表
    #[serde(rename = "triggerKeywords")]
    pub trigger_keywords: Vec<String>,
}

/// 挂载在查询结果上的动作项。
/// 服务于用户触发后的 Plugin::execute_action 执行流程。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultAction {
    // 这个是动作的唯一标识符，通常是一个字符串，由插件定义
    #[serde(rename = "id")]
    pub id: String,
    // 这个是动作的显示名称，用于展示在 UI 上
    #[serde(rename = "label")]
    pub label: String,
    // 这个是该选项的图标，用于展示在 UI 上
    #[serde(rename = "icon")]
    pub icon: IconRequest,
    // 是不是默认的动作，默认的动作会在用户直接按下回车时被触发
    #[serde(rename = "isDefault")]
    pub is_default: bool,
    /// 快捷键提示，格式如 "Shift+Enter"、"Ctrl+Enter"
    /// 前端根据此字段匹配修饰键到 action 的映射
    #[serde(rename = "shortcutKey")]
    pub shortcut_key: String,
}

/// 单个插件实例的静态元数据描述。
/// 服务于注册中心索引、触发词路由与插件发现/展示。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "version")]
    pub version: String,
    #[serde(rename = "description")]
    pub description: String,
    #[serde(rename = "author")]
    pub author: String,
    #[serde(rename = "triggerKeywords")]
    pub trigger_keywords: Vec<String>,
    #[serde(rename = "supportedOs")]
    pub supported_os: Vec<String>,
    #[serde(rename = "priority")]
    pub priority: i32,
}

/// 插件层统一错误类型。
/// 服务于生命周期/查询/动作/设置相关错误在宿主与插件间传播。
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("Plugin not found: {0}")]
    NotFound(String),

    #[error("Plugin initialization failed: {0}")]
    InitFailed(String),

    #[error("Query failed: {0}")]
    QueryFailed(String),

    #[error("Action execution failed: {0}")]
    ActionFailed(String),

    #[error("Invalid setting: {0}")]
    InvalidSetting(String),
}

#[cfg(test)]
mod tests {
    use super::{
        PanelInteraction, PanelKeyAction, PanelKeyBinding, PanelQueryTrigger, PluginContext,
        QueryChannel, QueryRevisionGate,
    };
    use serde_json::json;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;

    #[test]
    /// 验证门控：计数器未越过时当前查询仍有效，越过后失效。
    fn query_revision_gate_tracks_latest() {
        let latest = Arc::new(AtomicU64::new(2));
        let gate = QueryRevisionGate::new(2, latest.clone());
        assert!(gate.is_current(), "版本号与最新一致时应有效");
        assert_eq!(gate.revision(), 2);

        latest.fetch_add(1, Ordering::Relaxed);
        assert!(!gate.is_current(), "更新的查询到达后应失效");
    }

    #[test]
    /// 验证 PluginContext 门控默认行为与注入行为。
    fn plugin_context_gate_defaults_to_current() {
        let mut ctx = PluginContext::new("trace-1");
        assert!(ctx.is_query_current(), "无门控时应恒为最新");
        assert_eq!(ctx.query_revision(), 0);
        assert_eq!(
            ctx.query_channel,
            QueryChannel::Ui,
            "未显式注入时应缺省为 GUI 通道"
        );

        let latest = Arc::new(AtomicU64::new(1));
        ctx.set_query_revision_gate(QueryRevisionGate::new(1, latest));
        assert!(ctx.is_query_current());
    }

    #[test]
    /// 验证门控与句柄字段不参与序列化（不跨 RPC 传输），反序列化后为 None。
    fn plugin_context_skips_gate_in_serialization() {
        let mut ctx = PluginContext::new("trace-2");
        ctx.set_query_revision_gate(QueryRevisionGate::new(1, Arc::new(AtomicU64::new(1))));
        let json = serde_json::to_string(&ctx).expect("上下文应可序列化");
        assert!(
            !json.contains("revision") && !json.contains("gate"),
            "门控字段不应出现在序列化结果中: {}",
            json
        );
        assert!(
            !json.contains("handle"),
            "句柄字段不应出现在序列化结果中: {}",
            json
        );

        let roundtrip: PluginContext = serde_json::from_str(&json).expect("上下文应可反序列化");
        assert!(
            roundtrip.query_revision_gate.is_none(),
            "反序列化后门控应为 None"
        );
        assert!(roundtrip.is_query_current());
        assert_eq!(
            roundtrip.query_channel,
            QueryChannel::Ui,
            "通道字段应参与序列化且缺省为 GUI 通道"
        );
    }

    #[test]
    /// 验证面板交互策略的 JSON 字段名、枚举值和默认值。
    fn panel_interaction_serializes_with_stable_contract() {
        let interaction = PanelInteraction {
            query_trigger: PanelQueryTrigger::OnEnter,
            query_debounce_ms: 300,
            bindings: vec![PanelKeyBinding {
                key: "Enter".to_string(),
                action: PanelKeyAction::Confirm,
            }],
        };
        let value = serde_json::to_value(&interaction).expect("交互策略应可序列化");
        assert_eq!(
            value,
            json!({
                "queryTrigger": "onEnter",
                "queryDebounceMs": 300,
                // 完整传输契约：bindings 始终序列化（no-skip-serializing-if），缺省为空列表
                "bindings": [{"key": "Enter", "action": {"kind": "confirm"}}],
            })
        );

        let default_value: PanelInteraction =
            serde_json::from_value(json!({})).expect("缺失交互策略字段时应使用默认值");
        assert_eq!(default_value, PanelInteraction::default());
    }
}
