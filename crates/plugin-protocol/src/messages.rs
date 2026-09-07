use serde::{Deserialize, Serialize};
use zerolaunch_plugin_api::config::ComponentType;
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
    #[serde(rename = "pluginVersion")]
    pub plugin_version: String,
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
    #[serde(rename = "plugin")]
    Plugin {
        #[serde(rename = "triggerKeywords")]
        trigger_keywords: Vec<String>,
    },
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
// 协议版本偏离记录：`KeywordOptimizerInfo` 的载荷曾为 `usesContext: bool`，
// v1.0 迭代中改为 `inputSource: KeywordInputSource`（嵌套对象），属载荷级
// breaking change。决策：不 bump PROTOCOL_VERSION（bump 会拒绝全部现存旧
// SDK 插件），采用 1.x 内容忍 —— 老插件响应缺 `inputSource` 时宿主侧回退
// `Refined`；老宿主向新插件发 info 请求时，新插件响应含 `inputSource` 而
// 老宿主忽略未知字段。若未来允许载荷级不兼容变更，应 bump major 并拒绝旧版本。

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
    /// serde default：老宿主（不认识 inputSource 字段）向新 SDK 插件发请求时，
    /// 缺字段反序列化回退 Refined，避免 INVALID_PARAMS 导致优化器整体跳过。
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

// ─── host/icon.get ───────────────────────────────────────────────
// 返回值：图标字节的 base64 字符串。字节格式为 WebP（编码失败回退可能为
// PNG），消费方按字节头嗅探 MIME（RIFF....WEBP → image/webp，否则 image/png）。

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IconGetParams {
    #[serde(rename = "request")]
    pub request: serde_json::Value,
    #[serde(rename = "level")]
    pub level: String,
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
