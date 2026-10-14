use serde::{Deserialize, Serialize};
use zerolaunch_plugin_api::config::ComponentType;
use zerolaunch_plugin_api::host::PluginSdkConfig;
use zerolaunch_plugin_api::services::focus_monitor::FocusEvent;
use zerolaunch_plugin_api::services::hotkey::types::{HotkeyEvent, HotkeyEventFilter};
use zerolaunch_plugin_api::services::installation_monitor::types::InstallationEvent;
use zerolaunch_plugin_api::services::IconRequest;
use zerolaunch_plugin_api::{
    CandidateCacheSnapshot, CandidateId, ExecutionContext, KeywordInputSource, PluginContext,
    Query, ScoredCandidate, SearchCandidate, TargetType,
};

// ─── plugin/initialize ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeParams {
    #[serde(rename = "hostVersion")]
    pub host_version: String,
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    #[serde(rename = "dataDir")]
    pub data_dir: String,
    #[serde(rename = "logDir")]
    pub log_dir: String,
    #[serde(rename = "pluginId")]
    pub plugin_id: String,
    #[serde(rename = "locale")]
    pub locale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResult {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
}

// ─── plugin/get_components ───────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentDescriptor {
    #[serde(rename = "componentId")]
    pub component_id: String,
    #[serde(rename = "componentName")]
    pub component_name: String,
    #[serde(rename = "componentDescription", default)]
    pub component_description: String,
    #[serde(rename = "componentType")]
    pub component_type: ComponentType,
    #[serde(rename = "kind")]
    pub kind: ComponentKind,
    #[serde(rename = "priority", default = "default_component_priority")]
    pub priority: u32,
}

/// 默认组件优先级，与 `Configurable::priority()` 的默认值（50）一致。
fn default_component_priority() -> u32 {
    50
}

/// 与内置插件完全对等：Plugin / DataSource / ActionExecutor / SearchEngine /
/// ScoreBooster / KeywordOptimizer / KeywordInjector 均可由第三方插件提供，
/// 宿主按 kind 注册进对应管道（触发词路由 / 候选采集 / 执行器 / 搜索管道 / 候选管道）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ComponentKind {
    /// 主插件组件（插件级元数据由宿主从清单读取）。
    #[serde(rename = "plugin")]
    Plugin,
    #[serde(rename = "data_source")]
    DataSource,
    #[serde(rename = "action_executor")]
    ActionExecutor {
        #[serde(rename = "targetTypes")]
        target_types: Vec<TargetType>,
    },
    /// 搜索引擎 —— 参与搜索管道（与内置引擎互斥，仅允许一个启用）。
    #[serde(rename = "search_engine")]
    SearchEngine,
    /// 分数增强器 —— 在引擎打分后追加分数修正（多个可同时启用）。
    #[serde(rename = "score_booster")]
    ScoreBooster,
    /// 关键词优化器 —— 在候选管道中扩展搜索关键词。
    /// input_source / priority 为声明属性，经 keyword_optimizer_info RPC 拉取。
    #[serde(rename = "keyword_optimizer")]
    KeywordOptimizer,
    /// 关键词注入器 —— 基于候选项完整上下文注入额外关键词。
    #[serde(rename = "keyword_injector")]
    KeywordInjector,
}

// ─── plugin/get_settings_schema ──────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetSettingsSchemaParams {
    #[serde(rename = "componentId")]
    pub component_id: String,
}

// ─── plugin/get_settings ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetSettingsParams {
    #[serde(rename = "componentId")]
    pub component_id: String,
}

// ─── plugin/apply_settings ───────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplySettingsParams {
    #[serde(rename = "componentId")]
    pub component_id: String,
    #[serde(rename = "settings")]
    pub settings: serde_json::Value,
}

// ─── plugin/validate_settings ────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateSettingsParams {
    #[serde(rename = "componentId")]
    pub component_id: String,
    #[serde(rename = "settings")]
    pub settings: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateSettingsResult {
    #[serde(default)]
    pub error: Option<String>,
}

// ─── plugin/config_actions ───────────────────────────────────────
//
// 响应为裸数组 `Vec<ConfigActionDef>`（宿主 discover 流程以数组反序列化，
// 与 get_settings_schema 的裸数组约定一致）。

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigActionsParams {
    #[serde(rename = "componentId")]
    pub component_id: String,
}

// ─── plugin/execute_config_action ────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteConfigActionParams {
    #[serde(rename = "componentId")]
    pub component_id: String,
    #[serde(rename = "action")]
    pub action: String,
    #[serde(rename = "params")]
    pub params: serde_json::Value,
}

// ─── plugin/query ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryParams {
    #[serde(rename = "pluginId")]
    pub plugin_id: String,
    #[serde(rename = "ctx")]
    pub ctx: PluginContext,
    #[serde(rename = "query")]
    pub query: Query,
}

// ─── plugin/match_query ──────────────────────────────────────────

/// `plugin/match_query` 请求参数 —— 宿主路由阶段询问插件是否接管当前输入。
///
/// 使用范围：宿主（`RemoteComponent` → SDK 分发）；对全部启用中的行内插件调用。
/// 响应为布尔值（true = 接管）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchQueryParams {
    /// 用户在搜索栏中输入的原始字符串（未做小写化等预处理）。
    #[serde(rename = "rawQuery")]
    pub raw_query: String,
    /// 该插件声明的触发词（宿主从清单/代码元数据读取后随请求下发）。
    /// 用途：让插件的默认 `match_query` 实现能就地完成框架关键词判定，无需在插件代码里重复声明。
    #[serde(rename = "triggerKeywords")]
    pub trigger_keywords: Vec<String>,
}

// ─── plugin/execute_action ───────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteActionParams {
    #[serde(rename = "pluginId")]
    pub plugin_id: String,
    #[serde(rename = "ctx")]
    pub ctx: PluginContext,
    #[serde(rename = "actionId")]
    pub action_id: String,
    #[serde(rename = "payload")]
    pub payload: serde_json::Value,
}

// ─── plugin/fetch_candidates ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchCandidatesParams {
    #[serde(rename = "componentId")]
    pub component_id: String,
}

/// Serializable candidate payload returned by remote DataSource plugins.
/// The host reconstructs CachedCandidateData from these on receipt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchCandidatesResult {
    #[serde(rename = "candidates")]
    pub candidates: Vec<SearchCandidate>,
}

// ─── plugin/supported_target_types ───────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportedTargetTypesParams {
    #[serde(rename = "componentId")]
    pub component_id: String,
}

// ─── plugin/supported_actions ────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportedActionsParams {
    #[serde(rename = "componentId")]
    pub component_id: String,
    #[serde(rename = "targetType")]
    pub target_type: TargetType,
}

// ─── plugin/executor_execute ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutorExecuteParams {
    #[serde(rename = "componentId")]
    pub component_id: String,
    /// 完整执行上下文原样透传（与进程内 ActionExecutor 拿到的 ExecutionContext 一致：
    /// target / display_name / user_args / parameter_snapshot / locale）。
    #[serde(rename = "executionCtx")]
    pub execution_ctx: ExecutionContext,
    #[serde(rename = "actionId")]
    pub action_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutorExecuteResult {
    #[serde(default)]
    pub error: Option<String>,
}

// ─── plugin/calculate_scores ─────────────────────────────────────
//
// SearchEngine 组件：对缓存候选计算分数（含全量候选与查询词，宿主按
// ScoredCandidate 列表重建结果）。

/// plugin/calculate_scores 请求参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalculateScoresParams {
    /// 目标组件 id（SearchEngine 组件）。
    #[serde(rename = "componentId")]
    pub component_id: String,
    /// 全量候选快照（宿主 CachedCandidateData::to_data 导出，id 保真）。
    #[serde(rename = "candidates")]
    pub candidates: CandidateCacheSnapshot,
    /// 用户查询词。
    #[serde(rename = "query")]
    pub query: String,
}

// ─── plugin/booster_boost ────────────────────────────────────────
//
// ScoreBooster 组件：对已计算分数做增强，返回增强后的分数列表（索引与传入一致）。

/// plugin/booster_boost 请求参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoosterBoostParams {
    /// 目标组件 id（ScoreBooster 组件）。
    #[serde(rename = "componentId")]
    pub component_id: String,
    /// 全量候选快照（宿主 CachedCandidateData::to_data 导出，id 保真）。
    #[serde(rename = "candidates")]
    pub candidates: CandidateCacheSnapshot,
    /// 引擎计算出的当前分数列表。
    #[serde(rename = "scored")]
    pub scored: Vec<ScoredCandidate>,
    /// 用户查询词。
    #[serde(rename = "query")]
    pub query: String,
}

// ─── plugin/booster_record ───────────────────────────────────────
//
// ScoreBooster 组件：记录用户确认（选中候选 → 学习用户习惯）。

/// plugin/booster_record 请求参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoosterRecordParams {
    /// 目标组件 id（ScoreBooster 组件）。
    #[serde(rename = "componentId")]
    pub component_id: String,
    /// 被选中候选项的 id（宿主侧缓存内的原始 id）。
    #[serde(rename = "candidateId")]
    pub candidate_id: CandidateId,
    /// 全量候选快照（宿主 CachedCandidateData::to_data 导出，id 保真）。
    #[serde(rename = "candidates")]
    pub candidates: CandidateCacheSnapshot,
    /// 用户查询词。
    #[serde(rename = "query")]
    pub query: String,
}

// ─── plugin/keyword_optimizer_info ───────────────────────────────
//
// KeywordOptimizer 组件的声明属性（input_source / priority），
// 宿主在发现与设置变更时经本方法拉取并缓存。
//
// 响应为裸 `KeywordOptimizerInfo`。
//
// 载荷说明：`inputSource`（嵌套对象）取代了 1.x 的 `usesContext: bool`，属载荷级
// breaking change，已随协议 major 提升到 2.0 —— 1.x 插件在 initialize 握手期被拒绝。

/// KeywordOptimizer 组件的优化属性（经 keyword_optimizer_info RPC 拉取）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordOptimizerInfo {
    /// 优化器消费的关键词产物来源（分层 DAG-lite 输入层）。
    #[serde(rename = "inputSource")]
    pub input_source: KeywordInputSource,
    /// 链式执行优先级，小者先执行。
    #[serde(rename = "priority")]
    pub priority: u32,
}

/// plugin/keyword_optimizer_info 请求参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordOptimizerInfoParams {
    /// 目标组件 id（KeywordOptimizer 组件）。
    #[serde(rename = "componentId")]
    pub component_id: String,
}

// ─── plugin/keyword_optimize ─────────────────────────────────────
//
// KeywordOptimizer 组件：优化单个关键词（宿主按链序逐级调用）。
//
// 响应为裸 `Vec<String>`（优化生成的新关键词）。

/// plugin/keyword_optimize 请求参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordOptimizeParams {
    /// 目标组件 id（KeywordOptimizer 组件）。
    #[serde(rename = "componentId")]
    pub component_id: String,
    /// 待优化的关键词。
    #[serde(rename = "keyword")]
    pub keyword: String,
    /// 本次调用携带的来源层（供第三方优化器决策，宿主按声明来源调用）。
    /// serde default：字段缺失时反序列化回退 Refined，避免 INVALID_PARAMS
    /// 导致优化器整体跳过。
    #[serde(rename = "inputSource", default)]
    pub input_source: KeywordInputSource,
}

// KeywordInjector 组件：对单个候选注入关键词（与内置逐候选语义一致）。
//
// 响应为裸 `Vec<String>`。

/// plugin/keyword_inject 请求参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordInjectParams {
    /// 目标组件 id（KeywordInjector 组件）。
    #[serde(rename = "componentId")]
    pub component_id: String,
    /// 待注入的候选项（完整上下文：target、keywords 等）。
    #[serde(rename = "candidate")]
    pub candidate: SearchCandidate,
}

// ─── plugin/init ─────────────────────────────────────────────────
//
// 插件初始化钩子：宿主在组件注册完成后调用，通知插件进程完成初始化。
// 与 plugin/initialize（进程级握手）不同层：init 携带真实查询上下文
// （trace_id / locale），对应内置插件启动期的 Plugin::init。

/// plugin/init 请求参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitParams {
    /// 目标插件 id（宿主覆盖为 manifest 插件 id）。
    #[serde(rename = "pluginId")]
    pub plugin_id: String,
    /// 宿主构造的初始化上下文（真实 trace_id 与当前语言）。
    #[serde(rename = "ctx")]
    pub ctx: PluginContext,
}

// ─── plugin/interaction_policy ───────────────────────────────────
//
// 响应为裸 `PanelInteraction`（与 get_settings_schema 的裸数组约定一致）。
// 策略为插件级语义：宿主仅对 Plugin 种类组件调用；SDK 恒返回主插件策略。

/// plugin/interaction_policy 请求参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionPolicyParams {
    /// 目标组件 id（仅 Plugin 种类组件会收到该请求）。
    #[serde(rename = "componentId")]
    pub component_id: String,
}

// ─── plugin/get_default_enabled ──────────────────────────────────
//
// 响应为裸 `bool`。

/// plugin/get_default_enabled 请求参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetDefaultEnabledParams {
    /// 目标组件 id。
    #[serde(rename = "componentId")]
    pub component_id: String,
}

// ─── host/log ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogParams {
    #[serde(rename = "level")]
    pub level: String,
    #[serde(rename = "message")]
    pub message: String,
}

// ─── host/notify ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotifyParams {
    #[serde(rename = "title")]
    pub title: String,
    #[serde(rename = "message")]
    pub message: String,
}

// ─── host/shell.open ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellOpenParams {
    #[serde(rename = "target")]
    pub target: String,
}

// ─── host/shell.open_folder ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellOpenFolderParams {
    #[serde(rename = "path")]
    pub path: String,
}

// ─── host/shell.execute_elevation ────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellExecuteElevationParams {
    #[serde(rename = "path")]
    pub path: String,
}

// ─── host/shell.execute_command ──────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellExecuteCommandParams {
    #[serde(rename = "cmd")]
    pub cmd: String,
}

// ─── host/window.activate_by_process ─────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowActivateParams {
    #[serde(rename = "pid")]
    pub pid: u32,
}

// ─── host/icon.get / host/icon.extract / host/icon.refresh ───────
// 返回值为图标字节的 base64 字符串。字节格式为 WebP（编码失败回退可能为
// PNG），消费方按字节头嗅探 MIME（RIFF....WEBP → image/webp，否则 image/png）。
// icon.get 在提取失败时回退默认图标；extract / refresh 失败返回错误。

/// 图标请求参数（icon.get / extract / refresh 共用）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IconRequestParams {
    /// 图标提取请求（文件路径 / 网址 / 扩展名 / 已编码数据）。
    #[serde(rename = "request")]
    pub request: IconRequest,
}

// ─── host/path.resolve ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathResolveParams {
    #[serde(rename = "kind")]
    pub kind: String,
}

// ─── host/resource.upload ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUploadParams {
    #[serde(rename = "resourceId")]
    pub resource_id: String,
    #[serde(rename = "filePath")]
    pub file_path: String,
    #[serde(rename = "maxSize")]
    #[serde(default)]
    pub max_size: Option<usize>,
}

// ─── host/resource.put ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePutParams {
    #[serde(rename = "resourceId")]
    pub resource_id: String,
    #[serde(rename = "bytesB64")]
    pub bytes_b64: String,
}

// ─── host/resource.get ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceGetParams {
    #[serde(rename = "resourceId")]
    pub resource_id: String,
}

// ─── host/resource.delete ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceDeleteParams {
    #[serde(rename = "resourceId")]
    pub resource_id: String,
}

// ─── host/resource.list ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceListParams {}

// ─── host/parameter.resolve ──────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterResolveParams {
    #[serde(rename = "userArgs")]
    pub user_args: Vec<String>,
    /// 参数模板字符串（如 "https://google.com/search?q={query}"）。
    /// 命名注意：此字段是模板，不是插件 ID。字段曾名为 pluginId 但造成混淆，已更正。
    #[serde(rename = "template")]
    pub template: String,
}

// ─── host/sdk_config.update ──────────────────────────────────────

/// 更新插件 SDK 配置的参数。
///
/// 由插件的 `host/sdk_config.update` 调用携带；宿主将该配置应用到
/// 该插件的 PluginHandle（影响后续图标缓存等级等行为），立即生效。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdkConfigUpdateParams {
    /// 新的插件 SDK 配置；未设置字段表示沿用宿主当前值。
    #[serde(rename = "config")]
    pub config: PluginSdkConfig,
}

// ─── host/icon.override_cache ────────────────────────────────────

/// 覆盖图标缓存条目为自定义图标文件的参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IconOverrideCacheParams {
    /// 需要覆盖缓存图标的原始图标请求。
    #[serde(rename = "request")]
    pub request: IconRequest,
    /// 用户选择的自定义图标文件路径。
    #[serde(rename = "customIconPath")]
    pub custom_icon_path: String,
}

// ─── host/window.activate_by_process ─────────────────────────────

/// 按进程名激活窗口的参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowActivateByProcessParams {
    /// 进程名（如 "chrome.exe"）。
    #[serde(rename = "processName")]
    pub process_name: String,
}

// ─── host/window.activate_by_title ───────────────────────────────

/// 按窗口标题片段激活窗口的参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowActivateByTitleParams {
    /// 窗口标题片段。
    #[serde(rename = "title")]
    pub title: String,
}

// ─── host/clipboard.set_text ─────────────────────────────────────

/// 写入系统剪贴板的参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardSetTextParams {
    /// 待写入剪贴板的文本。
    #[serde(rename = "text")]
    pub text: String,
}

// ─── host/app.launch ─────────────────────────────────────────────
// 返回值为启动进程的 PID。

/// 启动已安装应用的参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppLaunchParams {
    /// 应用 ID（AppInfo.id / AppsFolder 条目 ID）。
    #[serde(rename = "appId")]
    pub app_id: String,
    /// 启动参数列表；None 表示不传参数。
    #[serde(rename = "args")]
    #[serde(default)]
    pub args: Option<Vec<String>>,
}

// ─── host/app.icon_path ──────────────────────────────────────────

/// 查询内置应用图标资源路径的参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppIconPathParams {
    /// 资源名称（如 "tray_icon"、"terminal"）。
    #[serde(rename = "name")]
    pub name: String,
}

// ─── host/path.resolve_lnk ───────────────────────────────────────

/// 解析 .lnk 快捷方式目标的参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathResolveLnkParams {
    /// .lnk 文件路径。
    #[serde(rename = "lnkPath")]
    pub lnk_path: String,
}

// ─── host/path.localized_names ───────────────────────────────────

/// 解析目录本地化文件名的参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathLocalizedNamesParams {
    /// 目标目录路径。
    #[serde(rename = "dirPath")]
    pub dir_path: String,
}

// ─── host/parameter.count_user / host/parameter.has_system ───────

/// 参数模板查询参数（统计用户参数 / 判断系统参数共用）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterTemplateParams {
    /// 参数模板字符串（如 "https://example.com?q={query}"）。
    #[serde(rename = "template")]
    pub template: String,
}

// ─── host/cache.put ──────────────────────────────────────────────

/// 写入插件本地缓存的参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachePutParams {
    /// 缓存域（插件内逻辑分组，需为非空且不含路径分隔符/..）。
    #[serde(rename = "domain")]
    pub domain: String,
    /// 缓存键（域内唯一，允许子路径但不得逃逸域目录）。
    #[serde(rename = "key")]
    pub key: String,
    /// 缓存字节数据的 base64 编码。
    #[serde(rename = "bytesB64")]
    pub bytes_b64: String,
}

// ─── host/cache.get ──────────────────────────────────────────────

/// 读取插件本地缓存的参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheGetParams {
    /// 缓存域。
    #[serde(rename = "domain")]
    pub domain: String,
    /// 缓存键。
    #[serde(rename = "key")]
    pub key: String,
}

// ─── host/cache.delete ───────────────────────────────────────────

/// 删除插件本地缓存条目的参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheDeleteParams {
    /// 缓存域。
    #[serde(rename = "domain")]
    pub domain: String,
    /// 缓存键。
    #[serde(rename = "key")]
    pub key: String,
}

// ─── host/cache.cleanup ──────────────────────────────────────────

/// 按条目数上限清理插件本地缓存域的参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheCleanupParams {
    /// 缓存域。
    #[serde(rename = "domain")]
    pub domain: String,
    /// 保留的最大条目数，超出部分按修改时间从旧到新删除。
    #[serde(rename = "maxEntries")]
    pub max_entries: usize,
}

// ─── host/timer.set_timeout ──────────────────────────────────────
// 返回值为宿主分配的定时器 ID（u64），到期由宿主推送 plugin/timer.fired。

/// 创建一次性定时器的参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimerSetTimeoutParams {
    /// 延迟毫秒数。
    #[serde(rename = "delayMs")]
    pub delay_ms: u64,
}

// ─── host/timer.set_interval ─────────────────────────────────────

/// 创建重复定时器的参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimerSetIntervalParams {
    /// 触发间隔毫秒数。
    #[serde(rename = "intervalMs")]
    pub interval_ms: u64,
}

// ─── host/timer.cancel ───────────────────────────────────────────

/// 取消指定定时器的参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimerCancelParams {
    /// 定时器 ID（host/timer.set_* 的返回值）。
    #[serde(rename = "id")]
    pub id: u64,
}

// ─── host/timer.cancel_all ───────────────────────────────────────

/// 取消本插件全部定时器的参数（无字段）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimerCancelAllParams {}

// ─── plugin/timer.fired ──────────────────────────────────────────

/// 宿主推送的定时器到期通知载荷。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimerFiredParams {
    /// 到期的定时器 ID。
    #[serde(rename = "id")]
    pub id: u64,
}

// ─── host/hotkey.subscribe ───────────────────────────────────────

/// 订阅按键事件的参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeySubscribeParams {
    /// 插件内订阅 ID（宿主按 "{plugin_id}:{id}" 前缀化，退订时用同一 ID）。
    #[serde(rename = "id")]
    pub id: String,
    /// 关注的事件过滤器（全部 / 指定快捷键 / 双击 Ctrl）。
    #[serde(rename = "filter")]
    pub filter: HotkeyEventFilter,
}

// ─── host/hotkey.unsubscribe ─────────────────────────────────────

/// 退订按键事件的参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeyUnsubscribeParams {
    /// 订阅时使用的插件内 ID。
    #[serde(rename = "id")]
    pub id: String,
}

// ─── host/installation.subscribe / unsubscribe ───────────────────

/// 订阅安装监控事件的参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallationSubscribeParams {
    /// 插件内订阅 ID。
    #[serde(rename = "id")]
    pub id: String,
}

/// 退订安装监控事件的参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallationUnsubscribeParams {
    /// 订阅时使用的插件内 ID。
    #[serde(rename = "id")]
    pub id: String,
}

// ─── host/focus.subscribe / unsubscribe ──────────────────────────

/// 订阅窗口焦点事件的参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusSubscribeParams {
    /// 插件内订阅 ID。
    #[serde(rename = "id")]
    pub id: String,
}

/// 退订窗口焦点事件的参数。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusUnsubscribeParams {
    /// 订阅时使用的插件内 ID。
    #[serde(rename = "id")]
    pub id: String,
}

// ─── plugin/hotkey.event / plugin/installation.event / plugin/focus.event ──
// 载荷携带订阅时的插件内 ID，使插件可把事件路由到对应订阅的处理函数。

/// 宿主推送的按键事件载荷。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeyEventParams {
    /// 订阅时使用的插件内 ID。
    #[serde(rename = "id")]
    pub id: String,
    /// 事件内容（全局快捷键 / 双击 Ctrl）。
    #[serde(rename = "event")]
    pub event: HotkeyEvent,
}

/// 宿主推送的安装监控事件载荷。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallationEventParams {
    /// 订阅时使用的插件内 ID。
    #[serde(rename = "id")]
    pub id: String,
    /// 事件内容（变化类型 + 变化路径）。
    #[serde(rename = "event")]
    pub event: InstallationEvent,
}

/// 宿主推送的窗口焦点事件载荷。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusEventParams {
    /// 订阅时使用的插件内 ID。
    #[serde(rename = "id")]
    pub id: String,
    /// 事件内容（当前仅失去焦点）。
    #[serde(rename = "event")]
    pub event: FocusEvent,
}
