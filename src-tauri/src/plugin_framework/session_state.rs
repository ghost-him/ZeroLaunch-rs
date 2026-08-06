//! 会话投影类型 —— 统一会话系统的契约核心。
//!
//! 会话（Session）由三部分组成：归属（`ActiveSession.plugin_id`：None = 宿主默认搜索，
//! Some(id) = 插件）、展示形态（PresentationMode）、会话代际（generation）。
//! 后端经 `SessionDispatcher` 维护权威投影，前端经 `session-state` 事件镜像投影。

use serde::Serialize;
use std::sync::Arc;
use zerolaunch_plugin_api::PanelInteraction;

/// 展示形态 —— 会话投影的 UI 侧描述，序列化键名遵循 serde-rename 规则（camelCase）。
///
/// `Search` 合并列表与空结果：空结果由响应 `results` 长度隐式表达，
/// 展示形态层面不区分。
/// 注意：序列化键名为 camelCase（跨 IPC 契约，与前端 `PresentationMode` 联合类型一致）；
/// `as_str()` 为 snake_case（仅服务于 CLI `/v1/session` 输出与日志，不跨 IPC）。
/// 变体注释中的键名即 `serde(rename)` 后的值。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum PresentationMode {
    /// 会话结束（无活动会话）。序列化键名：`"none"`。
    #[serde(rename = "none")]
    None,
    /// 默认搜索（列表 / 空结果）。序列化键名：`"search"`。
    #[serde(rename = "search")]
    Search,
    /// 行内参数输入（默认搜索子形态）。序列化键名：`"inlineParam"`。
    #[serde(rename = "inlineParam")]
    InlineParam,
    /// 参数面板（默认搜索子形态）。序列化键名：`"paramPanel"`。
    #[serde(rename = "paramPanel")]
    ParamPanel,
    /// 行内插件面板（保留搜索栏）。序列化键名：`"pluginPanel"`。
    #[serde(rename = "pluginPanel")]
    PluginPanel,
    /// 全页面插件面板（接管整个窗口）。序列化键名：`"pluginImmersive"`。
    #[serde(rename = "pluginImmersive")]
    PluginImmersive,
}

impl PresentationMode {
    /// 展示形态字符串（与前端词表一致；None 时为 "none"）。
    pub fn as_str(&self) -> &'static str {
        match self {
            PresentationMode::None => "none",
            PresentationMode::Search => "search",
            PresentationMode::InlineParam => "inline_param",
            PresentationMode::ParamPanel => "param_panel",
            PresentationMode::PluginPanel => "plugin_panel",
            PresentationMode::PluginImmersive => "plugin_immersive",
        }
    }
}

/// 插件面板信息（会话事件携带；子面板 panel_id 默认 "main"）。
#[derive(Debug, Clone, Serialize)]
pub struct PluginPanelInfo {
    #[serde(rename = "pluginId")]
    pub plugin_id: String,
    #[serde(rename = "panelId")]
    pub panel_id: String,
}

/// 会话状态事件载荷 —— 整个会话系统的唯一事件（事件名 `session-state`）。
///
/// 由 Dispatcher 在会话投影变化（路由/确认/reset）或插件面板路由命中时构造，
/// 经 bootstrap 注入的 emitter 推送；CLI 等无窗口场景不注入 emitter，不产生此事件。
/// 前端无条件接受（幂等），并按 `generation` 单调递增更新投影。
/// 归属信息不单独携带：插件归属时 `panel.plugin_id` 即归属 id，宿主归属时 `panel` 为 None。
#[derive(Debug, Clone, Serialize)]
pub struct SessionStateEvent {
    /// 会话代际：归属/形态变化时递增。
    #[serde(rename = "generation")]
    pub generation: u64,
    /// 展示形态。
    #[serde(rename = "presentation")]
    pub presentation: PresentationMode,
    /// 插件面板信息：Some = 插件的面板元数据（plugin_id 即会话归属）；None = 宿主面板（默认搜索归属）。
    #[serde(rename = "panel")]
    pub panel: Option<PluginPanelInfo>,
    /// 插件面板交互契约（含按键映射）：Some = 插件的按键声明；None = 宿主面板（默认搜索归属），由宿主提供默认键。
    #[serde(rename = "interaction")]
    pub interaction: Option<PanelInteraction>,
    /// 插件触发词列表，供前端「输入是否仍属于当前面板」的 IPC 前判定（镜像参数唯一来源）。
    #[serde(rename = "triggerKeywords", default)]
    pub trigger_keywords: Vec<String>,
}

/// 活动会话（Dispatcher 内部权威投影）。
#[derive(Debug, Clone)]
pub struct ActiveSession {
    /// 会话代际：归属/形态变化时递增。
    pub generation: u64,
    /// 会话归属：None = 宿主默认搜索（含行内参数/参数面板子状态）；Some(id) = 触发式插件。
    pub plugin_id: Option<String>,
    /// 当前展示形态。
    pub presentation: PresentationMode,
}

/// 会话状态推送回调 —— 由 bootstrap 拿到 AppHandle 后注入；
/// CLI 等无窗口场景不注入。
pub type SessionStateEmitter = Arc<dyn Fn(SessionStateEvent) + Send + Sync>;
