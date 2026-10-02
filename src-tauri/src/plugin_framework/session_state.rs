//! 会话投影类型 —— 统一会话系统的契约核心。
//!
//! 会话（Session）由三部分组成：归属与形态（`SessionOwner`：宿主默认搜索 / 插件面板）、
//! 输入匹配模型（`InputMatch`：前端本地镜像判定的依据）、会话代际（`generation`）。
//! 后端经 `SessionDispatcher` 维护权威投影，前端经 `session-state` 事件镜像投影；
//! 事件载荷与宿主身份共用同一套类型，故「宿主会话却带插件契约」这类组合不可表达。

use serde::Serialize;
use std::sync::Arc;
use zerolaunch_plugin_api::{PanelInteraction, ResultAction};

/// 宿主会话形态 —— 宿主（默认搜索）侧会话的展示方式。
///
/// 使用范围：`SessionOwner::Host` 与 `SessionStateEvent::Host`（跨 IPC，`view` 字段）；
/// `as_str()` 供 `bridge_query` 响应 `mode` / CLI `/v1/session` / 日志使用。
/// 两套词表各自唯一来源：serde 键名为 camelCase（跨 IPC 契约），`as_str()` 为 snake_case。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum HostView {
    /// 会话结束（无活动会话）；前端据此复位本地会话。serde 键名 `"none"`。
    #[serde(rename = "none")]
    None,
    /// 默认搜索 —— 列表与空结果合并（空结果由响应 `results` 长度隐式表达）。serde 键名 `"search"`。
    #[serde(rename = "search")]
    Search,
    /// 行内参数输入 —— 默认搜索子形态，等待用户为候选项补参数。serde 键名 `"inlineParam"`。
    #[serde(rename = "inlineParam")]
    InlineParam,
    /// 参数面板 —— 默认搜索子形态，候选项的参数面板。serde 键名 `"paramPanel"`。
    #[serde(rename = "paramPanel")]
    ParamPanel,
}

impl HostView {
    /// snake_case 形态词（`bridge_query` 响应 `mode` / CLI 输出 / 日志共用；
    /// 前端 `SessionMode` 按此词表判别）。
    pub fn as_str(&self) -> &'static str {
        match self {
            HostView::None => "none",
            HostView::Search => "search",
            HostView::InlineParam => "inline_param",
            HostView::ParamPanel => "param_panel",
        }
    }
}

/// 插件会话形态 —— 插件接管会话时的展示方式（由插件响应 `keep_search_bar` 决定）。
///
/// 使用范围：`PluginIdentity::view`、`SessionStateEvent::Plugin`（跨 IPC，`view` 字段）
/// 与插件唤醒日志；`as_str()` 供 `bridge_query` 响应 `mode` 使用。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum PluginView {
    /// 行内插件面板（保留搜索栏）。serde 键名 `"pluginPanel"`。
    #[serde(rename = "pluginPanel")]
    Panel,
    /// 全页面插件面板（接管整个窗口，隐藏搜索栏）。serde 键名 `"pluginImmersive"`。
    #[serde(rename = "pluginImmersive")]
    Immersive,
}

impl PluginView {
    /// snake_case 形态词（`bridge_query` 响应 `mode` / 日志共用）。
    pub fn as_str(&self) -> &'static str {
        match self {
            PluginView::Panel => "plugin_panel",
            PluginView::Immersive => "plugin_immersive",
        }
    }
}

/// 输入匹配模型 —— 前端「输入是否仍属于当前插件面板」的本地镜像依据（IPC 前的防抖/在途判定）。
///
/// 与触发词绑成同一个值：`Keywords` 自带触发词，故「有触发词却无判定模型」不可表达。
/// 使用范围：`PluginIdentity::input_match`（下发前端）与 `LocatedPlugin::match_model`
/// （路由裁决产物；`None` = 无裁决，如面板直调）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "model")]
pub enum InputMatch {
    /// 关键词镜像：前端按触发词本地判定（首词命中 + 空格 + 有剩余内容）。
    #[serde(rename = "keywords")]
    Keywords {
        /// 插件声明的触发关键词（大小写不敏感；前端取输入首词小写后比对）。
        /// 序列化键名 `triggerKeywords`。
        #[serde(rename = "triggerKeywords")]
        trigger_keywords: Vec<String>,
    },
    /// 插件自决：前端无本地谓词，非空输入一律视为仍属于本面板（粘性），
    /// 退出由后端归属变更（宿主会话事件）驱动。
    #[serde(rename = "custom")]
    Custom,
}

/// 插件会话身份 —— 归属 id / 面板形态 / 输入匹配模型三元组。
///
/// 使用范围：`SessionOwner::Plugin`（代际比较的唯一依据）与 `SessionStateEvent::Plugin`
/// （跨 IPC，字段扁平展开在事件顶层）。**不含**交互契约与渲染载荷：二者在投递时解析，
/// 可变且不进代际比较（否则交互配置变更会污染代际，使在途请求被误判为过期）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PluginIdentity {
    /// 会话归属的插件 id。序列化键名 `pluginId`。
    #[serde(rename = "pluginId")]
    pub plugin_id: String,
    /// 插件面板形态（行内/沉浸式）。序列化键名 `view`。
    #[serde(rename = "view")]
    pub view: PluginView,
    /// 输入匹配模型：`Some` = 前端可本地镜像判定；`None` = 无镜像谓词
    /// （插件声明空触发词且无路由裁决，如热键唤醒的无触发词插件）。
    /// 序列化键名 `inputMatch`，序列化为 `null`（不跳过）。
    #[serde(rename = "inputMatch")]
    pub input_match: Option<InputMatch>,
}

/// 会话归属 —— 决定「是否换会话」（会话代际比较）的全部事实。
///
/// 使用范围：`SessionDispatcher::active_session`（权威状态）与事件构造；
/// 不参与跨 IPC 序列化（跨 IPC 由 `SessionStateEvent` 承担）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionOwner {
    /// 宿主会话（默认搜索及其子形态）。
    Host(HostView),
    /// 插件会话（归属 id / 形态 / 输入匹配模型）。
    Plugin(PluginIdentity),
}

impl SessionOwner {
    /// 会话是否已结束（宿主形态 `None`）；确认校验与只读查询的判据。
    pub fn is_ended(&self) -> bool {
        matches!(self, SessionOwner::Host(HostView::None))
    }

    /// snake_case 形态词（CLI `/v1/session` 输出与日志共用，词表同 `as_str()`）。
    pub fn view_str(&self) -> &'static str {
        match self {
            SessionOwner::Host(view) => view.as_str(),
            SessionOwner::Plugin(identity) => identity.view.as_str(),
        }
    }
}

/// 动作的前端形状 —— 查询响应（列表动作/面板动作）与 `session-state` 事件共用的唯一编码。
/// icon 字符串化后与前端 `contract.ts` 的 `ResultAction.icon: string` 一致；源数据同为 `ResultAction`。
#[derive(Debug, Clone, Serialize)]
pub struct ResultActionDto {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "label")]
    pub label: String,
    #[serde(rename = "icon")]
    pub icon: String,
    #[serde(rename = "isDefault")]
    pub is_default: bool,
    #[serde(rename = "shortcutKey")]
    pub shortcut_key: String,
}

impl From<ResultAction> for ResultActionDto {
    fn from(action: ResultAction) -> Self {
        ResultActionDto {
            id: action.id,
            label: action.label,
            icon: action.icon.value().to_string(),
            is_default: action.is_default,
            shortcut_key: action.shortcut_key,
        }
    }
}

/// 插件面板渲染载荷 —— 热键唤醒推送时携带（会话事件仅此路径携带内容；
/// 关键词查询路径的载荷随 bridge_query 响应下发，不重复推送）。
#[derive(Debug, Clone, Serialize)]
pub struct PluginPanelContent {
    /// 面板类型标识，前端按此选择面板组件渲染。
    #[serde(rename = "panelType")]
    pub panel_type: String,
    /// 面板数据（自由 JSON，面板自行定义结构）。
    #[serde(rename = "data")]
    pub data: serde_json::Value,
    /// 面板动作列表（供 Enter 执行默认动作 / 面板内动作切换）。
    #[serde(rename = "actions")]
    pub actions: Vec<ResultActionDto>,
}

/// 会话状态事件载荷 —— 整个会话系统的唯一事件（事件名 `session-state`）。
///
/// 由 Dispatcher 在会话投影变化（路由/确认/reset）或插件面板命中/热键唤醒时构造，
/// 经 bootstrap 注入的 emitter 推送；CLI 等无窗口场景不注入 emitter，不产生此事件。
/// 前端无条件接受（幂等），并按 `generation` 单调递增更新投影。
/// 两类会话是两个不同的事实，故为枚举：宿主事件不含插件字段，插件事件的交互契约必定存在。
/// 外层判别键为 `kind`（`"host"` / `"plugin"`）。
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind")]
pub enum SessionStateEvent {
    /// 宿主会话事件（默认搜索及其子形态；`view = "none"` 即会话结束）。
    #[serde(rename = "host")]
    Host {
        /// 会话代际：归属/形态/输入匹配模型变化时递增。序列化键名 `generation`。
        #[serde(rename = "generation")]
        generation: u64,
        /// 宿主展示形态。序列化键名 `view`。
        #[serde(rename = "view")]
        view: HostView,
    },
    /// 插件会话事件：身份（字段扁平展开在顶层）+ 投递时解析的交互契约。
    #[serde(rename = "plugin")]
    Plugin {
        /// 会话代际。序列化键名 `generation`。
        #[serde(rename = "generation")]
        generation: u64,
        /// 插件会话身份（归属 id / 形态 / 输入匹配模型），序列化时扁平展开到事件顶层。
        #[serde(flatten)]
        identity: PluginIdentity,
        /// 插件面板交互契约（按键映射 / 查询触发方式）：插件归属下必定存在，
        /// 由插件 `interaction_policy()` 在投递时解析。序列化键名 `interaction`。
        #[serde(rename = "interaction")]
        interaction: PanelInteraction,
        /// 面板渲染载荷：仅热键唤醒投递携带（`Some`）；关键词查询路径为 `None`
        /// （载荷随 `bridge_query` 响应下发）。序列化键名 `panelContent`，序列化为 `null`（不跳过）；
        /// 装箱避免枚举体积被最大变体拖大（载荷仅在唤醒路径出现，常规路径只付一个空指针）。
        #[serde(rename = "panelContent")]
        panel_content: Option<Box<PluginPanelContent>>,
    },
}

/// 活动会话（Dispatcher 内部权威投影）。
///
/// 使用范围：仅 `SessionDispatcher` 维护与读取（代际校验、重新投递、只读查询）；
/// 前端不直接消费此类型（经 `SessionStateEvent` 镜像）。
#[derive(Debug, Clone)]
pub struct ActiveSession {
    /// 会话代际：`owner` 变化时递增；在途请求按此校验，过期即拒绝。
    pub generation: u64,
    /// 会话归属（变更检测的唯一依据；不含交互契约与渲染载荷——二者投递时解析）。
    pub owner: SessionOwner,
}

/// 会话状态推送回调 —— 由 bootstrap 拿到 AppHandle 后注入；
/// CLI 等无窗口场景不注入。
pub type SessionStateEmitter = Arc<dyn Fn(SessionStateEvent) + Send + Sync>;

#[cfg(test)]
mod tests {
    use super::*;

    /// 插件会话事件的线协议形状：判别键 `kind`、插件身份扁平展开、空载荷序列化为 `null`
    /// （前端契约依赖完整形状，不得跳过字段）。
    #[test]
    fn plugin_event_wire_shape() {
        let event = SessionStateEvent::Plugin {
            generation: 7,
            identity: PluginIdentity {
                plugin_id: "translator".to_string(),
                view: PluginView::Panel,
                input_match: Some(InputMatch::Keywords {
                    trigger_keywords: vec!["fy".to_string()],
                }),
            },
            interaction: PanelInteraction::default(),
            panel_content: None,
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["kind"], "plugin");
        assert_eq!(json["generation"], 7);
        assert_eq!(json["pluginId"], "translator");
        assert_eq!(json["view"], "pluginPanel");
        assert_eq!(json["inputMatch"]["model"], "keywords");
        assert_eq!(json["inputMatch"]["triggerKeywords"][0], "fy");
        assert!(json.get("interaction").is_some(), "交互契约必定存在");
        assert_eq!(json["panelContent"], serde_json::Value::Null);
    }

    /// 宿主会话事件形状：不含任何插件字段（字段级耦合由类型保证）。
    #[test]
    fn host_event_wire_shape() {
        let event = SessionStateEvent::Host {
            generation: 3,
            view: HostView::None,
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["kind"], "host");
        assert_eq!(json["generation"], 3);
        assert_eq!(json["view"], "none");
        assert!(json.get("pluginId").is_none());
        assert!(json.get("inputMatch").is_none());
        assert!(json.get("interaction").is_none());
        assert!(json.get("panelContent").is_none());
    }

    /// 形态词表：`as_str()` 为 snake_case（查询响应 `mode` / CLI），serde 键名为 camelCase（事件 `view`）。
    #[test]
    fn view_vocabularies() {
        assert_eq!(HostView::ParamPanel.as_str(), "param_panel");
        assert_eq!(PluginView::Immersive.as_str(), "plugin_immersive");
        assert_eq!(
            serde_json::to_value(HostView::InlineParam).unwrap(),
            "inlineParam"
        );
        assert_eq!(
            serde_json::to_value(PluginView::Immersive).unwrap(),
            "pluginImmersive"
        );
    }

    /// 自决模型的插件事件：`inputMatch` 为 `{"model":"custom"}`，无触发词字段。
    #[test]
    fn custom_match_wire_shape() {
        let event = SessionStateEvent::Plugin {
            generation: 1,
            identity: PluginIdentity {
                plugin_id: "detector".to_string(),
                view: PluginView::Immersive,
                input_match: Some(InputMatch::Custom),
            },
            interaction: PanelInteraction::default(),
            panel_content: Some(Box::new(PluginPanelContent {
                panel_type: "third-party:detector".to_string(),
                data: serde_json::json!({}),
                actions: Vec::new(),
            })),
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["inputMatch"]["model"], "custom");
        assert!(json["inputMatch"].get("triggerKeywords").is_none());
        assert_eq!(json["panelContent"]["panelType"], "third-party:detector");
    }
}
