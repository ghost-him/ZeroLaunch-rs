//! 搜索视图的数据模型与纯函数。

use std::sync::Arc;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use gpui_kit::component::input::InputState;
use gpui_kit::{Entity, Image, ImageFormat};
use serde_json::Value;
use zerolaunch_plugin_api::plugin::{ListItem, ResultAction};
use zerolaunch_plugin_api::services::IconRequest;
use zerolaunch_rs_lib::core::config::exclusion_settings::ExclusionKind;
use zerolaunch_rs_lib::plugin_framework::CandidateExcludeOptions;

/// 当前会话形态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// 默认搜索/空查询主页。
    Search,
    /// 行内参数输入。
    InlineParam,
    /// 参数面板。
    ParamPanel,
    /// 行内插件面板（保留搜索栏）。
    PluginPanel,
    /// 沉浸式插件面板（全窗口）。
    PluginImmersive,
}

/// 候选动作的展示数据。
#[derive(Debug, Clone)]
pub struct ActionInfo {
    /// 动作 id（回传内核执行）。
    pub id: String,
    /// 已本地化的动作文案。
    pub label: String,
    /// 是否为默认动作。
    pub is_default: bool,
    /// 快捷键提示（如 Shift+Enter）。
    pub shortcut_key: String,
}

impl From<ResultAction> for ActionInfo {
    fn from(action: ResultAction) -> Self {
        Self {
            id: action.id,
            label: action.label,
            is_default: action.is_default,
            shortcut_key: action.shortcut_key,
        }
    }
}

/// 一条候选结果。
#[derive(Clone)]
pub struct SearchItem {
    /// 候选 id。
    pub id: u64,
    /// 标题。
    pub title: String,
    /// 副标题（路径/URL/描述）。
    pub subtitle: String,
    /// 已解析图标。
    pub icon: Option<Arc<Image>>,
    /// 可执行动作。
    pub actions: Vec<ActionInfo>,
    /// 目标类型（Plugin 等）。
    pub target_type: String,
    /// 用户参数数量。
    pub user_arg_count: usize,
}

impl SearchItem {
    /// 从内核候选项构造展示数据。
    /// 参数：item - 内核候选项；icon - 已解析图标。
    /// 返回：视图使用的候选项。
    pub fn from_list_item(item: ListItem, icon: Option<Arc<Image>>) -> Self {
        Self {
            id: item.id,
            title: item.title,
            subtitle: item.subtitle,
            icon,
            actions: item.actions.into_iter().map(ActionInfo::from).collect(),
            target_type: item.target_type,
            user_arg_count: item.user_arg_count,
        }
    }

    /// 默认动作或当前动作索引对应的动作 ID。
    /// 参数：action_index - 当前选中的动作下标。
    /// 返回：动作 ID；无动作时 None。
    pub fn action_id(&self, action_index: usize) -> Option<&str> {
        self.actions
            .get(action_index.min(self.actions.len().saturating_sub(1)))
            .or_else(|| self.actions.iter().find(|action| action.is_default))
            .map(|action| action.id.as_str())
    }
}

/// 行内参数模式状态（输入复用主搜索框）。
#[derive(Debug, Clone)]
pub struct InlineState {
    /// 目标候选 id。
    pub candidate_id: u64,
    /// 命中的触发词。
    pub trigger_keyword: String,
    /// 必需参数数量。
    pub user_arg_count: usize,
}

/// 参数面板中的一个输入字段。
pub struct ParamField {
    /// 字段标签。
    pub label: String,
    /// 字段输入状态。
    pub input: Entity<InputState>,
}

/// 参数面板状态。
pub struct ParamState {
    /// 目标候选 id。
    pub candidate_id: u64,
    /// 目标候选标题。
    pub candidate_title: String,
    /// 参数字段。
    pub fields: Vec<ParamField>,
    /// 当前聚焦字段下标。
    pub focused: usize,
    /// 是否因缺参数被拒绝提交。
    pub invalid: bool,
}

/// 插件面板容器状态。
#[derive(Clone)]
pub struct PanelState {
    /// 规范化后的面板类型。
    pub panel_type: String,
    /// 面板自由数据。
    pub data: Value,
    /// 面板动作。
    pub actions: Vec<ActionInfo>,
    /// 面板所属插件 id。
    pub plugin_id: Option<String>,
}

/// 隐藏方式选择面板状态。
pub struct ExcludeState {
    /// 目标候选 id。
    pub candidate_id: u64,
    /// 目标候选标题。
    pub title: String,
    /// 后端推导的可用隐藏方式取值。
    pub options: CandidateExcludeOptions,
    /// 当前选中的隐藏方式。
    pub kind: ExclusionKind,
    /// 关键字输入框（Contains 方式使用）。
    pub keyword: Entity<InputState>,
}

/// 一次查询的完整结果。
pub struct QueryOutcome {
    /// 会话代际。
    pub generation: u64,
    /// 候选缓存世代。
    pub candidate_generation: u64,
    /// 会话形态。
    pub mode: Mode,
    /// 列表结果。
    pub results: Vec<SearchItem>,
    /// 行内参数状态。
    pub inline: Option<InlineState>,
    /// 插件面板状态。
    pub panel: Option<PanelState>,
    /// 尚未解析、需要后台补图的候选。
    pub pending_icons: Vec<(u64, IconRequest)>,
}

/// 可选的隐藏方式；无文件夹目标时不提供目录方式。
/// 参数：options - 后端推导的取值。
/// 返回：按展示顺序排列的方式列表。
pub fn available_exclude_kinds(options: &CandidateExcludeOptions) -> Vec<ExclusionKind> {
    if options.folder.is_none() {
        vec![ExclusionKind::Exact, ExclusionKind::Contains]
    } else {
        vec![
            ExclusionKind::Exact,
            ExclusionKind::PathPrefix,
            ExclusionKind::Contains,
        ]
    }
}

/// 行内参数解析：空格分隔，`\ ` 表示字面空格，`\\` 表示反斜杠。
/// 参数：input - 用户输入。
/// 返回：参数列表。
pub fn parse_inline_args(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\\' => match chars.peek().copied() {
                Some(' ') => {
                    current.push(' ');
                    chars.next();
                }
                Some('\\') => {
                    current.push('\\');
                    chars.next();
                }
                _ => current.push('\\'),
            },
            ' ' => {
                if !current.is_empty() {
                    args.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }
    if !current.is_empty() {
        args.push(current);
    }
    args
}

/// 把 data URL 解成 GPUI 图像。
/// 参数：data_url - 图标 data URL。
/// 返回：可渲染图像；格式不支持或解码失败时 None。
pub fn decode_icon(data_url: &str) -> Option<Arc<Image>> {
    let (meta, payload) = data_url.split_once(',')?;
    if !meta.contains("base64") {
        return None;
    }
    let bytes = BASE64.decode(payload).ok()?;
    let format = if meta.contains("svg") {
        ImageFormat::Svg
    } else if meta.contains("jpeg") || meta.contains("jpg") {
        ImageFormat::Jpeg
    } else if meta.contains("webp") {
        ImageFormat::Webp
    } else if meta.contains("gif") {
        ImageFormat::Gif
    } else if meta.contains("bmp") {
        ImageFormat::Bmp
    } else if meta.contains("ico") || meta.contains("x-icon") {
        ImageFormat::Ico
    } else {
        ImageFormat::Png
    };
    Some(Arc::new(Image::from_bytes(format, bytes)))
}
