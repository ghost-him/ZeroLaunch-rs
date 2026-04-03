use crate::plugin_system::cached_candidate::CachedCandidateData;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub type CandidateId = u64;

/// 启动方法类型枚举，用于 Launcher 注册和查找
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum LaunchMethodType {
    Path,
    PackageFamilyName,
    File,
    Url,
    Command,
    BuiltinCommand,
}

/// 这个表示是使用什么启动方法来完成对于该搜索候选项的启动的
#[derive(Debug, Clone, Serialize, Deserialize, Eq, Hash, PartialEq)]
pub enum LaunchMethod {
    /// 表示使用路径启动
    Path(String),
    /// 表示使用包家族名称启动
    PackageFamilyName(String),
    /// 表示使用文件启动
    File(String),
    /// 表示使用URL启动
    Url(String),
    /// 表示使用命令启动
    Command(String),
    /// 表示使用内置命令启动
    BuiltinCommand(String),
}

impl LaunchMethod {
    /// 获取启动方法的类型
    pub fn method_type(&self) -> LaunchMethodType {
        match self {
            LaunchMethod::Path(_) => LaunchMethodType::Path,
            LaunchMethod::PackageFamilyName(_) => LaunchMethodType::PackageFamilyName,
            LaunchMethod::File(_) => LaunchMethodType::File,
            LaunchMethod::Url(_) => LaunchMethodType::Url,
            LaunchMethod::Command(_) => LaunchMethodType::Command,
            LaunchMethod::BuiltinCommand(_) => LaunchMethodType::BuiltinCommand,
        }
    }

    /// 获取启动方法的载荷（路径、URL、命令等）
    pub fn payload(&self) -> &str {
        match self {
            LaunchMethod::Path(s) => s,
            LaunchMethod::PackageFamilyName(s) => s,
            LaunchMethod::File(s) => s,
            LaunchMethod::Url(s) => s,
            LaunchMethod::Command(s) => s,
            LaunchMethod::BuiltinCommand(s) => s,
        }
    }
}

// 这个是一个搜索候选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchCandidate {
    // 候选项的唯一标识符
    pub id: CandidateId,
    // 表示用于显示在搜索结果中的名称
    pub name: String,
    // 表示用于显示在搜索结果中的图标
    pub icon: String,
    // 表示应该怎么启动这个候选项
    pub launch_method: LaunchMethod,
    // 表示该候选项的关键词，即怎么可以确认用户想要启动这个候选项
    pub keywords: Vec<String>,
    // 固定的权重偏移，用于在计算分数时考虑该候选项的固定权重。由每个数据源来控制各自的权重
    pub bias: f64,
}

// 这个是一个搜索候选项的详细分数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreDetail {
    // 基础分
    pub score: f64,
    // 当前权重分
    pub weight: f64,
    // 这个是什么分，以及这个分的来源
    pub description: String,
}

// 这个是一个搜索候选项的分数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredCandidate {
    // 表示该候选项的分数
    pub candidate_id: CandidateId,
    // 表示该候选项的分数
    pub score: f64,
    //表示该候选项得来的详细的分数, score = sum(detailed_score * detailed_weight)
    pub detailed_score: Vec<ScoreDetail>,
}

/// 组件类型枚举，用于区分不同类型的可配置组件
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum ComponentType {
    DataSource,
    KeywordOptimizer,
    SearchEngine,
    ScoreBooster,
    Launcher,
    Plugin,
    Core,
}

/// 配置错误类型
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Configuration validation failed: {0}")]
    ValidationFailed(String),

    #[error("Invalid setting value for key '{key}': {message}")]
    InvalidValue { key: String, message: String },

    #[error("Setting not found: {0}")]
    NotFound(String),

    #[error("Configuration apply failed: {0}")]
    ApplyFailed(String),
}

/// 组件配置项的字段定义。
/// 用于描述配置项的核心属性，可被 SettingDefinition 和 ArrayItem::Object 复用。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDefinition {
    pub key: String,
    pub label: String,
    pub description: String,
    pub setting_type: SettingType,
    pub default_value: serde_json::Value,
    pub visible: bool,
    pub editable: bool,
}

impl Default for FieldDefinition {
    fn default() -> Self {
        Self {
            key: String::new(),
            label: String::new(),
            description: String::new(),
            setting_type: SettingType::Text,
            default_value: serde_json::Value::Null,
            visible: true,
            editable: true,
        }
    }
}

/// 组件配置项的声明式定义。
/// 服务于设置存储与动态设置界面生成。
///
/// 字段语义说明：
/// - `field.default_value`: 整个设置项的默认值（如整个数组的默认内容）
/// - `FieldDefinition.default_value`（在 ArrayItem::Object 内）: 新增一行对象时，该字段的默认值模板
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SettingDefinition {
    pub field: FieldDefinition,
    pub group: Option<String>,
    pub order: u32,
}

/// 数组元素的 UI 渲染提示。
/// 用于指导前端如何渲染数组类型的配置项。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum ArrayUiHint {
    #[default]
    Default,
    Table,
    MasterDetail,
    Tags,
}

/// 原始类型枚举，用于数组元素的类型定义。
/// 与 SettingType 类似，但不包含复合类型（Array）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PrimitiveType {
    Text,
    Number {
        min: Option<f64>,
        max: Option<f64>,
        step: Option<f64>,
    },
    Boolean,
    Select {
        options: Vec<String>,
    },
    Path {
        mode: PathMode,
    },
    Color,
}

/// 数组元素类型定义。
/// 用于区分数组元素是原始类型还是对象类型。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArrayItem {
    Primitive(PrimitiveType),
    Object(Vec<FieldDefinition>),
}

/// 组件设置项的输入控件类型。
/// 服务于设置表单渲染与取值校验。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SettingType {
    Text,
    Number {
        min: Option<f64>,
        max: Option<f64>,
        step: Option<f64>,
    },
    Boolean,
    Select {
        options: Vec<String>,
    },
    Path {
        mode: PathMode,
    },
    Color,
    Json,
    Array {
        item: ArrayItem,
        min_items: Option<usize>,
        max_items: Option<usize>,
        ui_hint: ArrayUiHint,
    },
}

/// 路径选择模式
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PathMode {
    File,
    Directory,
}

/// 所有可配置组件都需实现的核心契约。
/// 提供组件标识、配置定义、配置读写和配置变更回调能力。
pub trait Configurable: Send + Sync {
    fn component_id(&self) -> &str;
    fn component_name(&self) -> &str;
    fn component_type(&self) -> ComponentType;

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![]
    }

    fn get_settings(&self) -> serde_json::Value {
        serde_json::Value::Object(serde_json::Map::new())
    }

    fn apply_settings(&mut self, settings: serde_json::Value) -> Result<(), ConfigError> {
        let _ = settings;
        Ok(())
    }

    fn validate_settings(&self, settings: &serde_json::Value) -> Result<(), ConfigError> {
        let _ = settings;
        Ok(())
    }

    fn get_default_settings(&self) -> serde_json::Value {
        let schema = self.setting_schema();
        let mut map = serde_json::Map::new();
        for def in schema {
            if !def.field.default_value.is_null() {
                map.insert(def.field.key.clone(), def.field.default_value.clone());
            }
        }
        serde_json::Value::Object(map)
    }

    fn on_settings_changed(&self) {}
}

// 表示一个数据源
pub trait DataSource: Configurable {
    fn fetch_candidates(&self) -> CachedCandidateData;
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
    fn record(&self, candidate: &mut ScoredCandidate, query: &str);
    // 根据用户历史输入的查询与选择的候选项，优化当前查询所得到的所有候选项的分数
    fn boost(&self, candidates: &mut Vec<ScoredCandidate>);
}

/// 启动器错误类型
#[derive(Debug, thiserror::Error)]
pub enum LaunchError {
    #[error("Launch failed: {0}")]
    Failed(String),

    #[error("Launcher not found for method: {0:?}")]
    NotFound(LaunchMethodType),
}

/// 表示一个启动器，用于启动搜索候选项
/// 每个 Launcher 只负责一种 LaunchMethodType
pub trait Launcher: Send + Sync {
    /// 返回该 Launcher 支持的启动方法类型
    fn supported_method(&self) -> LaunchMethodType;

    /// 执行启动
    fn launch(&self, method: &LaunchMethod) -> Result<(), LaunchError>;
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
    pub id: String,
    pub raw_query: String,
    pub search_term: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryResponse {
    // 目前先实现该内容
    List {
        results: Vec<ListItem>,
    },
    // 该内容暂时先不实现，后续如果有需要了再来实现它
    CustomPanel {
        panel_type: String,         // "notebook", "todo", etc.
        data: serde_json::Value,    // 插件自定义数据
        actions: Vec<ResultAction>, // 可选的全局动作
    },
    // 该内容暂时先不实现，后续如果有需要了再来实现它
    WebView {
        url: String,
        width: Option<u32>,
        height: Option<u32>,
    },
    Empty,
}

/// 插件返回给宿主的搜索结果项。
/// 服务于结果聚合、排序与 UI 渲染。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListItem {
    // 这个是候选项的唯一标识符
    pub id: CandidateId,
    pub title: String,
    pub subtitle: String,
    pub icon: String,
    pub score: f64,
    // 一个动作列表中只可以有一个默认动作，默认动作会在用户直接按下回车时被触发（由程序员保证）
    pub actions: Vec<ResultAction>,
}

/// 挂载在查询结果上的动作项。
/// 服务于用户触发后的 Plugin::execute_action 执行流程。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultAction {
    // 这个是动作的唯一标识符，通常是一个字符串，由插件定义
    pub id: String,
    // 这个是动作的显示名称，用于展示在 UI 上
    pub label: String,
    // 这个是该选项的图标，用于展示在 UI 上
    pub icon: String,
    // 是不是默认的动作，默认的动作会在用户直接按下回车时被触发
    pub is_default: bool,
}

/// 单个插件实例的静态元数据描述。
/// 服务于注册中心索引、触发词路由与插件发现/展示。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub trigger_keywords: Vec<String>,
    pub supported_os: Vec<String>,
    pub priority: i32,
}

/// 所有插件对象都需实现的核心契约。
/// 服务于插件生命周期管理、查询处理与动作执行。
/// 配置管理能力由 Configurable trait 提供。
#[async_trait]
pub trait Plugin: Configurable {
    fn metadata(&self) -> &PluginMetadata;

    async fn init(&self, ctx: &PluginContext, api: Arc<dyn PluginAPI>) -> Result<(), PluginError>;

    async fn query(&self, ctx: &PluginContext, query: &Query)
        -> Result<QueryResponse, PluginError>;

    async fn execute_action(
        &self,
        ctx: &PluginContext,
        action_id: &str,
        payload: serde_json::Value,
    ) -> Result<(), PluginError>;
}

/// 宿主向插件暴露能力的契约。
/// 服务于日志、通知、设置读写以及 UI 回调等横切能力。
#[async_trait]
pub trait PluginAPI: Send + Sync {
    async fn log(&self, ctx: &PluginContext, level: LogLevel, message: &str);

    async fn notify(&self, ctx: &PluginContext, title: &str, message: &str);

    async fn get_setting(&self, plugin_id: &str, key: &str) -> Option<String>;

    async fn set_setting(&self, plugin_id: &str, key: &str, value: &str);

    async fn refresh_programs(&self);

    async fn hide_window(&self);
}

/// PluginAPI::log 使用的日志级别。
#[derive(Debug, Clone)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
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
