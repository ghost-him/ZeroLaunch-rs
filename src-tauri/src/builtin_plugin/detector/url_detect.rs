use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
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
use crate::core::config::setting_builders::SchemaBuilder;
use crate::plugin_framework::builtin_registry::PluginEntry;

/// 网址形态检测插件的配置结构。
///
/// 数据来源：`ConfigManager` 中组件 `url-detect` 的 settings 段（键名 snake_case）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlDetectSettings {
    /// 裸域名（无 scheme，如 `github.com`）是否视为网址。
    /// true（默认）= 裸域名也接管；false = 仅带 scheme 的输入接管。
    #[serde(
        rename = "bare_domain_enabled",
        default = "default_bare_domain_enabled"
    )]
    pub bare_domain_enabled: bool,
}

/// 裸域名判定的默认值：开启（用户输入 `github.com` 即视为网址）。
fn default_bare_domain_enabled() -> bool {
    true
}

impl Default for UrlDetectSettings {
    fn default() -> Self {
        Self {
            bare_domain_enabled: default_bare_domain_enabled(),
        }
    }
}

/// 网址形态检测插件 —— 输入形态像网址时接管查询，用默认浏览器打开。
///
/// 使用范围：内置插件（经 `PluginEntry` 由 builtin_registry 注册），由 `SessionDispatcher`
/// 查询路由经 `Plugin::match_query` 自定义判定接管。**不补全 scheme**：裸域名按原样交给系统 shell
/// （与"运行"对话框行为一致），由用户为自己的输入负责。
pub struct UrlDetectPlugin {
    /// 组件身份（id、名称、描述、类型、优先级）。
    core: ComponentCore,
    /// 插件级元数据（触发词留空：判定由 `match_query` 自行实现）。
    metadata: PluginMetadata,
    /// PluginHandle（init 时发放），供 shell 能力访问。
    handle: RwLock<Option<Arc<PluginHandle>>>,
    /// 最近一次渲染的目标，供 execute_action 执行（面板动作通道不带面板数据）。
    last_target: RwLock<Option<String>>,
    /// 设置（内部可变性：apply_settings 时写入，match_query/query 时读取）。
    settings: RwLock<UrlDetectSettings>,
}

impl Default for UrlDetectPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl UrlDetectPlugin {
    /// 创建插件实例（元数据与组件身份均使用 i18n key，前端按 key-or-literal 渲染）。
    pub fn new() -> Self {
        Self {
            core: ComponentCore::new(
                "url-detect".to_string(),
                t_key!("url-detect", "name").to_string(),
                t_key!("url-detect", "description").to_string(),
                ComponentType::Plugin,
                11,
            ),
            metadata: PluginMetadata {
                id: "url-detect".to_string(),
                name: t_key!("url-detect", "name").to_string(),
                // 内置插件无独立版本/作者（随应用分发），UI 按内置标识展示
                version: String::new(),
                description: t_key!("url-detect", "description").to_string(),
                author: String::new(),
                // 触发词留空：网址形态无法用静态触发词表达，判定由 match_query 自定义实现。
                trigger_keywords: Vec::new(),
                // 网址形态判定与平台无关
                supported_os: vec![
                    "windows".to_string(),
                    "macos".to_string(),
                    "linux".to_string(),
                ],
                // 路由优先级：数值小者优先；排在 path-detect(10) 之后
                priority: 11,
                kind: PluginKind::Builtin,
                hotkey: None,
                icon: None,
                mode: PluginMode::Inline,
            },
            handle: RwLock::new(None),
            last_target: RwLock::new(None),
            settings: RwLock::new(UrlDetectSettings::default()),
        }
    }
}

/// 带 scheme 的网址前缀（大小写不敏感）。
const URL_SCHEMES: [&str; 4] = ["http://", "https://", "ftp://", "file://"];

/// 常见文件扩展名 —— 用于排除"看起来像域名"的文件名（`snipaste.exe`、`node.js`）。
/// 命中即不视为裸域名（这类输入应交给搜索/路径检测）。
const FILE_EXTENSION_DENY: [&str; 34] = [
    "exe", "lnk", "bat", "cmd", "ps1", "msi", "dll", "sys", "ini", "log", "cfg", "txt", "md",
    "json", "toml", "yaml", "yml", "rs", "ts", "js", "tsx", "jsx", "py", "java", "c", "cpp", "h",
    "go", "rb", "php", "html", "css", "zip", "rar",
];

/// 常见顶级域 —— 单点域名（两段标签）只有命中此表才视为网址；
/// 三段及以上标签（如 `www.foo.bar`）不要求命中，以覆盖新顶级域。
const COMMON_TLDS: [&str; 40] = [
    "com", "org", "net", "edu", "gov", "mil", "int", "io", "dev", "app", "ai", "cn", "uk", "jp",
    "de", "fr", "ru", "br", "in", "au", "ca", "kr", "tw", "hk", "sg", "us", "me", "co", "info",
    "biz", "tv", "cc", "xyz", "top", "site", "online", "store", "tech", "cloud", "sh",
];

/// 网址形态判定（纯语法，无 IO）。
///
/// 命中任一形态即视为网址：
/// - 带 scheme（`http://` / `https://` / `ftp://` / `file://`，大小写不敏感）；
/// - 裸域名（仅当 `bare_domain_enabled`）：无空格、不含反斜杠、形如 `host.tld[:port][/path]`，
///   末段为字母且不在文件扩展名黑名单；两段标签时顶级域须命中常见表，三段及以上放宽。
pub(crate) fn looks_like_url(raw: &str, bare_domain_enabled: bool) -> bool {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return false;
    }
    let lower = trimmed.to_ascii_lowercase();
    if URL_SCHEMES.iter().any(|scheme| lower.starts_with(scheme)) {
        return true;
    }
    if !bare_domain_enabled {
        return false;
    }
    is_bare_domain(trimmed)
}

/// 裸域形态判定：标签结构与顶级域双重校验，排除文件名与含空格输入。
fn is_bare_domain(text: &str) -> bool {
    if text.contains(char::is_whitespace) || text.contains('\\') {
        return false;
    }
    // 去掉路径/查询/锚点后缀后只看主机段
    let host = text.split(['/', '?', '#']).next().unwrap_or("");
    if host.is_empty() {
        return false;
    }
    // 可选端口：仅允许数字端口
    let host = match host.rsplit_once(':') {
        Some((head, port)) => {
            if port.is_empty() || !port.chars().all(|c| c.is_ascii_digit()) {
                return false;
            }
            head
        }
        None => host,
    };
    let labels: Vec<&str> = host.split('.').collect();
    if labels.len() < 2 {
        return false;
    }
    let valid_label = |label: &str| {
        !label.is_empty()
            && label.len() <= 63
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
    };
    if !labels.iter().all(|label| valid_label(label)) {
        return false;
    }
    let tld = labels[labels.len() - 1].to_ascii_lowercase();
    if tld.len() < 2 || tld.len() > 24 || !tld.chars().all(|c| c.is_ascii_alphabetic()) {
        return false;
    }
    // 文件扩展名黑名单优先于域名判定：`main.rs` / `snipaste.exe` / `foo.bar.exe` 这类
    // 输入与"裸域名"在语法上不可区分，一律不视为网址（`https://docs.rs` 之类的显式
    // scheme 写法不受影响）。
    if FILE_EXTENSION_DENY.contains(&tld.as_str()) {
        return false;
    }
    // 两段标签（`github.com`）要求顶级域常见；三段及以上（`www.foo.xyz`）放宽，
    // 以覆盖新顶级域与多级子域。
    labels.len() > 2 || COMMON_TLDS.contains(&tld.as_str())
}

#[async_trait]
impl Configurable for UrlDetectPlugin {
    fn core(&self) -> &ComponentCore {
        &self.core
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        vec![SchemaBuilder::boolean(
            "bare_domain_enabled",
            t_key!("url-detect", "fields.bare_domain_enabled.label"),
            t_key!("url-detect", "fields.bare_domain_enabled.desc"),
        )
        .order(1)
        .default(true)
        .build()]
    }

    fn get_settings(&self) -> serde_json::Value {
        serde_json::to_value(&*self.settings.read()).unwrap_or_default()
    }

    async fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        let parsed: UrlDetectSettings = serde_json::from_value(settings).unwrap_or_default();
        *self.settings.write() = parsed;
        Ok(())
    }

    fn default_enabled(&self) -> bool {
        true
    }
}

#[async_trait]
impl Plugin for UrlDetectPlugin {
    /// 保存服务句柄（execute_action 经句柄访问 shell 能力）。
    async fn init(
        &self,
        _ctx: &PluginContext,
        handle: Option<Arc<PluginHandle>>,
    ) -> Result<(), PluginError> {
        *self.handle.write() = handle;
        Ok(())
    }

    /// 查询匹配（框架默认的关键词判定不适用，本插件自定义）：输入"已提交"（含空格，且尾部空格
    /// 或成对引号形态）且形态像网址时接管。网址不含空格（引号形态除外）。
    /// 纯语法判定，无 IO；裸域名开关读自设置。
    async fn match_query(&self, raw_query: &str, _declared_trigger_keywords: &[String]) -> bool {
        let Some(committed) = committed_input(raw_query) else {
            return false;
        };
        if committed.content.contains(char::is_whitespace) {
            return false;
        }
        let bare_domain_enabled = self.settings.read().bare_domain_enabled;
        looks_like_url(&committed.content, bare_domain_enabled)
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

    /// 渲染接管面板：展示目标网址与打开动作。
    /// 仅 GUI 通道且查询仍最新时缓存目标，供 execute_action 使用（面板动作通道不带面板数据）。
    async fn query(
        &self,
        ctx: &PluginContext,
        query: &Query,
    ) -> Result<QueryResponse, PluginError> {
        // 网址区分大小写（路径段），取原始输入归一化（去尾空格/去引号）而非小写化的 search_term
        let target = committed_input(&query.raw_query)
            .map(|committed| committed.content)
            .unwrap_or_else(|| query.raw_query.trim().to_string());
        if ctx.is_query_current() && ctx.query_channel == QueryChannel::Ui {
            *self.last_target.write() = Some(target.clone());
        }
        Ok(QueryResponse::CustomPanel {
            panel_type: PANEL_TYPE.to_string(),
            data: json!({
                "kind": "url",
                "target": target.clone(),
                "title": t_key!("smart-target", "openUrl"),
                "subtitle": target,
                "icon": serde_json::Value::Null,
            }),
            actions: vec![ResultAction {
                id: "execute".to_string(),
                label: t_key!("smart-target", "actions.open").to_string(),
                icon: IconRequest::Path(String::new()),
                is_default: true,
                shortcut_key: "Enter".to_string(),
            }],
            keep_search_bar: true,
        })
    }

    /// 执行面板动作：用系统默认方式打开网址（不补全 scheme，交由 shell 处理）。
    async fn execute_action(
        &self,
        _ctx: &PluginContext,
        action_id: &str,
        _payload: serde_json::Value,
    ) -> Result<(), PluginError> {
        let target =
            self.last_target.read().clone().ok_or_else(|| {
                PluginError::ActionFailed("目标网址不可用，请重新输入".to_string())
            })?;
        let handle = self
            .handle
            .read()
            .clone()
            .ok_or_else(|| PluginError::ActionFailed("插件服务句柄不可用".to_string()))?;
        match action_id {
            "execute" => handle
                .shell_open(OpenTarget::Url(target))
                .await
                .map_err(|e| PluginError::ActionFailed(format!("打开网址失败: {}", e))),
            _ => Err(PluginError::ActionFailed(format!(
                "Unknown action: {}",
                action_id
            ))),
        }
    }
}

fn build_url_detect_plugin() -> (Arc<dyn Configurable>, Arc<dyn Plugin>, PluginMetadata) {
    let plugin = Arc::new(UrlDetectPlugin::new());
    let metadata = plugin.metadata.clone();
    let configurable: Arc<dyn Configurable> = plugin.clone();
    let plugin: Arc<dyn Plugin> = plugin;
    (configurable, plugin, metadata)
}

::inventory::submit! {
    PluginEntry {
        component_id: "url-detect",
        priority: 11,
        factory: build_url_detect_plugin
    }
}

#[cfg(test)]
mod tests {
    use super::{committed_input, looks_like_url};

    /// 带 scheme 一律命中（大小写不敏感），与裸域名开关无关，但仍需"已提交"。
    #[test]
    fn matches_scheme_urls() {
        for raw in [
            "https://example.com ",
            "HTTP://Example.COM/path ",
            "http://localhost:8080 ",
            "ftp://files.example.com ",
            "file:///C:/tmp/a.txt ",
        ] {
            assert!(looks_like_committed_url(raw, true), "应判定为网址: {raw}");
            assert!(
                looks_like_committed_url(raw, false),
                "带 scheme 不应受裸域名开关影响: {raw}"
            );
        }
        // 未提交（无尾空格、非引号）不接管
        assert!(!looks_like_committed_url("https://example.com", true));
    }

    /// 裸域名：常见顶级域与多段域名命中；未提交、文件名、含空格、单段、非法端口不命中。
    #[test]
    fn matches_bare_domains() {
        for raw in [
            "github.com ",
            "www.google.com ",
            "example.com/path?q=1 ",
            "npmjs.com:443 ",
            "a.b.xyz ",
            // 引号形态须内含空格才提交（宿主前置门要求输入含空格）
            "\"example.com/path?q=1\" ",
        ] {
            assert!(looks_like_committed_url(raw, true), "应判定为网址: {raw}");
        }
        for raw in [
            "github.com",
            // 无空格的引号形态不提交（宿主不会询问）
            "\"github.com\"",
            "snipaste.exe ",
            "node.js ",
            "main.rs ",
            // 文件扩展名黑名单优先：`docs.rs` 与 Rust 源文件后缀同形，按不接管处理
            "docs.rs ",
            "foo.bar.exe ",
            "visual studio code ",
            "localhost ",
            "foo.invalidtld ",
            "foo.com:abc ",
            "foo.com: ",
            "-foo.com ",
            "foo-.com ",
            "example.com\\\\share ",
            "C:\\Users ",
        ] {
            assert!(
                !looks_like_committed_url(raw, true),
                "不应判定为网址: {raw}"
            );
        }
    }

    /// 关闭裸域名开关后，仅带 scheme 的输入接管。
    #[test]
    fn bare_domain_toggle_disables_shape_detection() {
        assert!(!looks_like_committed_url("github.com ", false));
        assert!(looks_like_committed_url("https://github.com ", false));
    }

    /// 把插件的匹配规则（提交 + 无空格约束 + 形态判定）整体跑一遍，供上面的用例复用。
    fn looks_like_committed_url(raw: &str, bare_domain_enabled: bool) -> bool {
        let Some(committed) = committed_input(raw) else {
            return false;
        };
        if committed.content.contains(char::is_whitespace) {
            return false;
        }
        looks_like_url(&committed.content, bare_domain_enabled)
    }
}
