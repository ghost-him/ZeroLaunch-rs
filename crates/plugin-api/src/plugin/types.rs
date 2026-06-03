use crate::config::Configurable;
use crate::plugin::cached_candidate::CachedCandidateData;
use crate::services::icon_request::IconRequest;
use crate::services::parameter::types::ParameterSnapshot;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub type CandidateId = u64;

/// 执行目标类型枚举，用于 ActionExecutor 注册和查找
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
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
    //表示该候选项得来的详细的分数, score = sum(detailed_score * detailed_weight)
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

/// 请求级上下文，在宿主与插件之间共享。
/// 服务于插件生命周期/查询/动作调用，并携带日志关联 ID。
#[derive(Debug, Clone)]
pub struct PluginContext {
    // 当前的请求 ID
    pub trace_id: String,
    // 当前的请求 ID
    pub query_id: Option<String>,
    // 处理当前请求的插件 ID
    pub plugin_id: Option<String>,
}

impl PluginContext {
    pub fn new(trace_id: &str) -> Self {
        Self {
            trace_id: trace_id.to_string(),
            query_id: None,
            plugin_id: None,
        }
    }

    pub fn with_query(&mut self, query_id: String) {
        self.query_id = Some(query_id);
    }

    pub fn with_plugin_id(&mut self, plugin_id: String) {
        self.plugin_id = Some(plugin_id);
    }
}

/// 发送给插件查询处理器的标准化查询载荷。
/// 服务于查询分发和插件侧搜索逻辑。
#[derive(Debug, Clone)]
pub struct Query {
    /// 本次查询的唯一标识，取自 bridge_query 中生成的 trace_id，用于日志关联和插件上下文。
    pub id: String,
    /// 用户在搜索栏中输入的原始字符串，未经任何处理，用于触发器匹配和日志记录。
    pub raw_query: String,
    /// 派生自 raw_query 的搜索词。普通搜索为全小写形式，插件模式为剥离触发关键词后的剩余部分。
    pub search_term: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryResponse {
    #[serde(rename = "list")]
    List {
        #[serde(rename = "results")]
        results: Vec<ListItem>,
    },
    #[serde(rename = "customPanel")]
    CustomPanel {
        #[serde(rename = "panelType")]
        panel_type: String,
        #[serde(rename = "data")]
        data: serde_json::Value,
        #[serde(rename = "actions")]
        actions: Vec<ResultAction>,
        #[serde(rename = "keepSearchBar")]
        keep_search_bar: bool,
    },
    #[serde(rename = "empty")]
    Empty,
    /// 行内参数模式：后端检测到触发关键词+空格后自动进入。
    /// 前端据此清空搜索栏并展示参数输入 UI。
    #[serde(rename = "inlineParam")]
    InlineParam {
        #[serde(rename = "candidateId")]
        candidate_id: CandidateId,
        #[serde(rename = "triggerKeyword")]
        trigger_keyword: String,
        #[serde(rename = "userArgCount")]
        user_arg_count: usize,
    },
}

/// route_confirm 的执行结果。
/// Executed 表示已执行完成；EnterParamPanel 表示候选项需要参数但未提供。
#[derive(Debug, Clone)]
pub enum ConfirmResult {
    Executed,
    EnterParamPanel {
        candidate_id: CandidateId,
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
