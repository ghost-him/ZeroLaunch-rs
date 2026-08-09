use serde::{Deserialize, Serialize};
use zerolaunch_plugin_api::config::ComponentType;
use zerolaunch_plugin_api::{ExecutionContext, PluginContext, Query, SearchCandidate, TargetType};

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
    pub priority: i32,
}

/// 默认组件优先级，与 `Configurable::priority()` 的默认值（50）一致。
fn default_component_priority() -> i32 {
    50
}

/// 第三方插件可声明的组件种类。
///
/// 第一版只开放 Plugin / DataSource / ActionExecutor。
/// TODO: 后续版本开放 KeywordOptimizer / SearchEngine / ScoreBooster，
/// 届时在此枚举中新增对应 variant，并同步更新 REQUIRED_PROVIDES_VALUES。
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
