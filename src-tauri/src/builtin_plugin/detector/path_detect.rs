use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use zerolaunch_plugin_api::common::ImageUtils;
use zerolaunch_plugin_api::config::{
    ComponentCore, ComponentType, ConfigError, Configurable, SettingDefinition,
};
use zerolaunch_plugin_api::host::{OpenTarget, PluginHandle};
use zerolaunch_plugin_api::services::IconRequest;
use zerolaunch_plugin_api::{
    PanelInteraction, PanelKeyAction, PanelKeyBinding, Plugin, PluginContext, PluginError,
    PluginKind, PluginMetadata, PluginMode, Query, QueryChannel, QueryResponse, ResultAction,
};

use super::{committed_input, PANEL_TYPE};
use crate::plugin_framework::builtin_registry::PluginEntry;

/// 路径形态检测插件的配置结构（当前无用户可配置项，仅用于占位与持久化兼容）。
///
/// 数据来源：`ConfigManager` 中组件 `path-detect` 的 settings 段（无字段）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PathDetectSettings {}

/// 路径形态检测插件 —— 输入形态像本地路径时接管查询，提供"打开/打开所在文件夹"。
///
/// 使用范围：内置插件（经 `PluginEntry` 由 builtin_registry 注册），由 `SessionDispatcher`
/// 查询路由经 `Plugin::match_query` 自定义判定接管。判定为纯形态判定：**不校验路径是否存在**，
/// 不存在的路径仍接管，失败由执行器在打开时报错。
pub struct PathDetectPlugin {
    /// 组件身份（id、名称、描述、类型、优先级）。
    core: ComponentCore,
    /// 插件级元数据（触发词留空：判定由 `match_query` 自行实现）。
    metadata: PluginMetadata,
    /// PluginHandle（init 时发放），供图标提取与 shell 能力访问。
    handle: RwLock<Option<Arc<PluginHandle>>>,
    /// 最近一次渲染的目标（展开环境变量后），供 execute_action 执行。
    /// 面板动作通道只回传动作 id（不携带面板数据），沿用内置插件缓存末次结果的既有约定。
    last_target: RwLock<Option<String>>,
    /// 设置（内部可变性：apply_settings 时写入）。
    settings: RwLock<PathDetectSettings>,
}

impl Default for PathDetectPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl PathDetectPlugin {
    /// 创建插件实例（元数据与组件身份均使用 i18n key，前端按 key-or-literal 渲染）。
    pub fn new() -> Self {
        Self {
            core: ComponentCore::new(
                "path-detect".to_string(),
                t_key!("path-detect", "name").to_string(),
                t_key!("path-detect", "description").to_string(),
                ComponentType::Plugin,
                10,
            ),
            metadata: PluginMetadata {
                id: "path-detect".to_string(),
                name: t_key!("path-detect", "name").to_string(),
                // 内置插件无独立版本/作者（随应用分发），UI 按内置标识展示
                version: String::new(),
                description: t_key!("path-detect", "description").to_string(),
                author: String::new(),
                // 触发词留空：路径形态无法用静态触发词表达，判定由 match_query 自定义实现。
                trigger_keywords: Vec::new(),
                supported_os: vec!["windows".to_string()],
                // 路由优先级：数值小者优先（与搜索管道/组件排序约定一致）
                priority: 10,
                kind: PluginKind::Builtin,
                hotkey: None,
                icon: None,
                mode: PluginMode::Inline,
            },
            handle: RwLock::new(None),
            last_target: RwLock::new(None),
            settings: RwLock::new(PathDetectSettings::default()),
        }
    }

    /// 通用文件夹图标（data URL）；句柄缺失或提取失败返回 None（前端渲染兜底图标）。
    async fn folder_icon(&self) -> Option<String> {
        let handle = self.handle.read().clone()?;
        let data = handle
            .get_icon_or_default(IconRequest::Extension("folder".to_string()))
            .await;
        Some(ImageUtils::to_data_url(&data))
    }
}

/// 路径形态判定（纯语法，无 IO）。
///
/// 命中任一形态即视为路径：
/// - 盘符绝对路径 `X:\` / `X:/`；UNC `\\server\share` 或 `//server/share`；
/// - 相对路径 `.\` / `..\` / `./` / `../`；
/// - 环境变量前缀 `%VAR%` 或 `%VAR%\...`（变量名仅允许字母数字与下划线）。
pub(crate) fn looks_like_path(raw: &str) -> bool {
    let trimmed = raw.trim();
    let mut chars = trimmed.chars();
    let (first, second) = (chars.next(), chars.next());
    if first.is_none() {
        return false;
    }
    // 盘符绝对路径：字母 + ':' + 分隔符
    if let (Some(drive), Some(':')) = (first, second) {
        if drive.is_ascii_alphabetic() && matches!(chars.next(), Some('\\') | Some('/')) {
            return true;
        }
    }
    if trimmed.starts_with("\\\\")
        || trimmed.starts_with("//")
        || trimmed.starts_with(".\\")
        || trimmed.starts_with("./")
        || trimmed.starts_with("..\\")
        || trimmed.starts_with("../")
    {
        return true;
    }
    env_var_prefix(trimmed)
}

/// 判断字符串是否以 `%VAR%` 环境变量形式开头（`%` 内为合法变量名）。
fn env_var_prefix(text: &str) -> bool {
    if !text.starts_with('%') {
        return false;
    }
    let mut parts = text.splitn(3, '%');
    let _leading = parts.next();
    match (parts.next(), parts.next()) {
        (Some(name), Some(_rest)) => {
            // 首字符须为字母或下划线：排除 `%1%`/`%2BAD%` 这类占位符与序号写法
            name.chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        }
        _ => false,
    }
}

/// 展开 `%VAR%` 形式的环境变量；未定义或非法形式原样保留。
///
/// 仅在 `query`/`execute_action` 阶段调用（含环境读取，非匹配期）。
pub(crate) fn expand_env_vars(input: &str) -> String {
    if !input.contains('%') {
        return input.to_string();
    }
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('%') {
            Some(end) => {
                let name = &after[..end];
                let valid =
                    !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
                match (valid, std::env::var(name)) {
                    (true, Ok(value)) => out.push_str(&value),
                    (true, Err(_)) => {
                        out.push('%');
                        out.push_str(name);
                        out.push('%');
                    }
                    (false, _) => {
                        out.push('%');
                        out.push_str(name);
                        out.push('%');
                    }
                }
                rest = &after[end + 1..];
            }
            None => {
                out.push('%');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

#[async_trait]
impl Configurable for PathDetectPlugin {
    fn core(&self) -> &ComponentCore {
        &self.core
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        // 无用户可配置项：是否检测由插件启用状态控制
        Vec::new()
    }

    fn get_settings(&self) -> serde_json::Value {
        serde_json::to_value(&*self.settings.read()).unwrap_or_default()
    }

    async fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        let parsed: PathDetectSettings = serde_json::from_value(settings).unwrap_or_default();
        *self.settings.write() = parsed;
        Ok(())
    }

    fn default_enabled(&self) -> bool {
        true
    }
}

#[async_trait]
impl Plugin for PathDetectPlugin {
    /// 保存服务句柄（execute_action/图标提取经句柄访问平台能力）。
    async fn init(
        &self,
        _ctx: &PluginContext,
        handle: Option<Arc<PluginHandle>>,
    ) -> Result<(), PluginError> {
        *self.handle.write() = handle;
        Ok(())
    }

    /// 查询匹配（框架默认的关键词判定不适用，本插件自定义）：输入"已提交"（含空格，且尾部空格
    /// 或成对引号形态）且形态像本地路径时接管。非引号内容按"路径不含空格"处理，
    /// 引号形态（如 `"C:\Program Files"`）允许内含空格。纯语法判定，无 IO。
    async fn match_query(&self, raw_query: &str, _declared_trigger_keywords: &[String]) -> bool {
        let Some(committed) = committed_input(raw_query) else {
            return false;
        };
        if !committed.quoted && committed.content.contains(char::is_whitespace) {
            return false;
        }
        looks_like_path(&committed.content)
    }

    /// 面板按键契约：Enter 执行默认动作（打开），Escape 退出回默认搜索。
    fn interaction_policy(&self) -> PanelInteraction {
        PanelInteraction {
            bindings: vec![
                PanelKeyBinding {
                    key: "Enter".to_string(),
                    action: PanelKeyAction::Confirm,
                },
                PanelKeyBinding {
                    key: "Escape".to_string(),
                    action: PanelKeyAction::GoBack,
                },
            ],
            ..Default::default()
        }
    }

    /// 渲染接管面板：展示归一化后的目标（去尾空格 / 去引号 / 展开环境变量）与执行动作。
    /// 仅 GUI 通道且查询仍最新时缓存目标，供 execute_action 使用（面板动作通道不带面板数据）。
    async fn query(
        &self,
        ctx: &PluginContext,
        query: &Query,
    ) -> Result<QueryResponse, PluginError> {
        let content = committed_input(&query.raw_query)
            .map(|committed| committed.content)
            .unwrap_or_else(|| query.raw_query.trim().to_string());
        let target = expand_env_vars(&content);
        if ctx.is_query_current() && ctx.query_channel == QueryChannel::Ui {
            *self.last_target.write() = Some(target.clone());
        }
        let icon = self.folder_icon().await;
        Ok(QueryResponse::CustomPanel {
            panel_type: PANEL_TYPE.to_string(),
            data: json!({
                "kind": "path",
                "target": target.clone(),
                "title": t_key!("smart-target", "openPath"),
                "subtitle": target,
                "icon": icon,
            }),
            actions: vec![
                ResultAction {
                    id: "execute".to_string(),
                    label: t_key!("smart-target", "actions.open").to_string(),
                    icon: IconRequest::Path(String::new()),
                    is_default: true,
                    shortcut_key: "Enter".to_string(),
                },
                ResultAction {
                    id: "open_folder".to_string(),
                    label: t_key!("smart-target", "actions.openFolder").to_string(),
                    icon: IconRequest::Path(String::new()),
                    is_default: false,
                    shortcut_key: String::new(),
                },
            ],
            keep_search_bar: true,
        })
    }

    /// 执行面板动作：打开目标（默认）或在文件管理器中打开其所在目录。
    async fn execute_action(
        &self,
        _ctx: &PluginContext,
        action_id: &str,
        _payload: serde_json::Value,
    ) -> Result<(), PluginError> {
        let target =
            self.last_target.read().clone().ok_or_else(|| {
                PluginError::ActionFailed("目标路径不可用，请重新输入".to_string())
            })?;
        let handle = self
            .handle
            .read()
            .clone()
            .ok_or_else(|| PluginError::ActionFailed("插件服务句柄不可用".to_string()))?;
        match action_id {
            "execute" => handle
                .shell_open(OpenTarget::File(target))
                .await
                .map_err(|e| PluginError::ActionFailed(format!("打开路径失败: {}", e))),
            "open_folder" => handle
                .shell_open_folder(&target)
                .await
                .map_err(|e| PluginError::ActionFailed(format!("打开所在文件夹失败: {}", e))),
            _ => Err(PluginError::ActionFailed(format!(
                "Unknown action: {}",
                action_id
            ))),
        }
    }
}

fn build_path_detect_plugin() -> (Arc<dyn Configurable>, Arc<dyn Plugin>, PluginMetadata) {
    let plugin = Arc::new(PathDetectPlugin::new());
    let metadata = plugin.metadata.clone();
    let configurable: Arc<dyn Configurable> = plugin.clone();
    let plugin: Arc<dyn Plugin> = plugin;
    (configurable, plugin, metadata)
}

::inventory::submit! {
    PluginEntry {
        component_id: "path-detect",
        priority: 10,
        factory: build_path_detect_plugin
    }
}

#[cfg(test)]
mod tests {
    use super::{committed_input, expand_env_vars, looks_like_path};

    /// 命中形态：已提交（含空格且尾空格 / 引号）的盘符、UNC、相对路径、环境变量路径。
    #[test]
    fn matches_path_shapes() {
        for raw in [
            "C:\\ ",
            "C:\\Users\\ ",
            "d:/projects/foo ",
            "\\\\server\\share ",
            "//server/share ",
            ".\\build ",
            "../src ",
            "%APPDATA% ",
            "%USERPROFILE%\\Desktop ",
            "  C:\\Users  ",
            // 引号形态（含空格路径）无需尾空格（内容含空格，故满足宿主的前置门）
            "\"C:\\Program Files\"",
            "\"C:\\Program Files\" ",
        ] {
            assert!(looks_like_committed_path(raw), "应判定为路径: {raw}");
        }
    }

    /// 不命中：未提交（无尾空格且非引号）、未加引号却含空格、非路径文本、URL。
    #[test]
    fn rejects_non_path_shapes() {
        for raw in [
            "",
            "   ",
            // 未按空格提交
            "C:\\",
            "C:\\Users\\",
            "\\\\server\\share",
            // 无空格的引号形态不提交（宿主前置门要求输入含空格）
            "\"C:\\Users\"",
            // 未加引号却含空格
            "C:\\Program Files ",
            "C:\\Program Files",
            // 非路径文本
            "visual studio code ",
            "C:",
            "1:\\foo ",
            "https://example.com ",
            "example.com/path ",
            "%",
            "%%",
            "%1BAD% ",
        ] {
            assert!(!looks_like_committed_path(raw), "不应判定为路径: {raw}");
        }
    }

    /// 把插件的匹配规则（提交 + 引号/空格约束 + 形态判定）整体跑一遍，供上面的用例复用。
    fn looks_like_committed_path(raw: &str) -> bool {
        let Some(committed) = committed_input(raw) else {
            return false;
        };
        if !committed.quoted && committed.content.contains(char::is_whitespace) {
            return false;
        }
        looks_like_path(&committed.content)
    }

    /// 环境变量展开：已定义变量替换为取值，未定义/非法形式原样保留。
    #[test]
    fn expands_env_vars() {
        let raw = format!("%{0}%\\Desktop", "PATH");
        let expanded = expand_env_vars(&raw);
        assert!(!expanded.starts_with('%'), "已定义变量应被展开: {expanded}");
        assert!(expanded.ends_with("\\Desktop"));

        assert_eq!(expand_env_vars("C:\\plain"), "C:\\plain");
        assert_eq!(
            expand_env_vars("%NOT_DEFINED_XYZ%\\a"),
            "%NOT_DEFINED_XYZ%\\a"
        );
        assert_eq!(expand_env_vars("%BAD-NAME%"), "%BAD-NAME%");
    }
}
