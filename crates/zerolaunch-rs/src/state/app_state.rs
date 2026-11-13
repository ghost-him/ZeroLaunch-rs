use crate::core::cli_token::CliToken;
use crate::core::config::ConfigManager;
use crate::core::host::{Host, NoopHost};
use crate::core::i18n::I18nManager;
use crate::core::model::ModelManager;
use crate::plugin_framework::plugin_manager::PluginManager;
use crate::plugin_framework::PluginRegistry;
use crate::plugin_framework::SessionDispatcher;
use crate::sdk::HostApi;
use crate::updater::UpdateChecker;
use crate::utils::waiting_hashmap::AsyncWaitingHashMap;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use zerolaunch_plugin_api::host::PluginHandle;
use zerolaunch_plugin_api::PluginMode;

pub struct AppState {
    session_dispatcher: Arc<SessionDispatcher>,
    config_manager: RwLock<Option<Arc<ConfigManager>>>,
    host: RwLock<Arc<dyn Host>>,
    waiting_hashmap: Arc<AsyncWaitingHashMap<String, Vec<(String, String)>>>,
    game_mode: RwLock<bool>,
    host_api: RwLock<Option<Arc<HostApi>>>,
    core_handle: RwLock<Option<Arc<PluginHandle>>>,
    inspector: RwLock<Option<Arc<crate::plugin_framework::inspector::Inspector>>>,
    /// PluginManager — 插件身份与生命周期的统一入口
    plugin_manager: RwLock<Option<Arc<PluginManager>>>,
    /// CLI server token (cached for the `cli_get_info` IPC command).
    cli_token: RwLock<Option<CliToken>>,
    /// 后端翻译服务（内置语言包 + 插件翻译目录）
    i18n_manager: RwLock<Option<Arc<I18nManager>>>,
    /// 模型管理器（聚合提供方并按 model_id 路由）
    model_manager: Arc<ModelManager>,
    /// 版本更新检查器（托盘、启动自检与设置页共用同一份状态）
    update_checker: Arc<UpdateChecker>,
}
impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        let plugin_registry = Arc::new(PluginRegistry::new());
        let session_dispatcher = Arc::new(SessionDispatcher::new(plugin_registry));

        AppState {
            session_dispatcher,
            config_manager: RwLock::new(None),
            host: RwLock::new(Arc::new(NoopHost::default())),
            waiting_hashmap: Arc::new(AsyncWaitingHashMap::new()),
            game_mode: RwLock::new(false),
            host_api: RwLock::new(None),
            core_handle: RwLock::new(None),
            inspector: RwLock::new(None),
            plugin_manager: RwLock::new(None),
            cli_token: RwLock::new(None),
            i18n_manager: RwLock::new(None),
            model_manager: Arc::new(ModelManager::new()),
            update_checker: Arc::new(UpdateChecker::new()),
        }
    }

    pub fn get_session_dispatcher(&self) -> &Arc<SessionDispatcher> {
        &self.session_dispatcher
    }

    pub fn get_config_manager(&self) -> Arc<ConfigManager> {
        self.config_manager
            .read()
            .as_ref()
            .cloned()
            .expect("config manager not initialized")
    }

    pub fn set_config_manager(&self, config_manager: Arc<ConfigManager>) {
        *self.config_manager.write() = Some(config_manager);
    }

    /// 当前宿主端口（默认为无窗口实现；GPUI 宿主启动时替换）。
    pub fn get_host(&self) -> Arc<dyn Host> {
        self.host.read().clone()
    }

    /// 安装宿主实现（GPUI / 测试）。
    pub fn set_host(&self, host: Arc<dyn Host>) {
        *self.host.write() = host;
    }

    pub fn get_waiting_hashmap(&self) -> Arc<AsyncWaitingHashMap<String, Vec<(String, String)>>> {
        self.waiting_hashmap.clone()
    }

    pub fn set_game_mode(&self, game_mode: bool) {
        *self.game_mode.write() = game_mode;
    }

    pub fn get_game_mode(&self) -> bool {
        *self.game_mode.read()
    }

    pub fn get_host_api(&self) -> Arc<HostApi> {
        self.host_api
            .read()
            .as_ref()
            .cloned()
            .expect("host_api not initialized")
    }

    pub fn set_host_api(&self, host_api: Arc<HostApi>) {
        *self.host_api.write() = Some(host_api);
    }

    pub fn get_core_handle(&self) -> Arc<PluginHandle> {
        self.core_handle
            .read()
            .as_ref()
            .cloned()
            .expect("core_handle not initialized")
    }

    pub fn set_core_handle(&self, handle: Arc<PluginHandle>) {
        *self.core_handle.write() = Some(handle);
    }

    pub fn get_inspector(&self) -> Option<Arc<crate::plugin_framework::inspector::Inspector>> {
        self.inspector.read().clone()
    }

    pub fn set_inspector(&self, inspector: Arc<crate::plugin_framework::inspector::Inspector>) {
        *self.inspector.write() = Some(inspector);
    }

    /// 检查调试模式是否开启。
    /// 直接从 ConfigManager 读取 general-config.is_debug_mode，无需维护独立状态。
    pub fn is_debug_mode(&self) -> bool {
        self.config_manager
            .read()
            .as_ref()
            .and_then(|cm| {
                cm.get_settings("general-config")
                    .and_then(|v| v.get("is_debug_mode")?.as_bool())
            })
            .unwrap_or(false)
    }

    /// 系统托盘图标是否显示。
    /// 直接从 ConfigManager 读取 general-config.is_show_tray_icon，无需维护独立状态。
    pub fn is_show_tray_icon(&self) -> bool {
        self.config_manager
            .read()
            .as_ref()
            .and_then(|cm| cm.get_component_setting("general-config", "is_show_tray_icon"))
            .and_then(|v| v.as_bool())
            .unwrap_or(true)
    }

    pub fn get_plugin_manager(&self) -> Arc<PluginManager> {
        self.plugin_manager
            .read()
            .as_ref()
            .cloned()
            .expect("plugin_manager not initialized")
    }

    pub fn set_plugin_manager(&self, manager: Arc<PluginManager>) {
        *self.plugin_manager.write() = Some(manager);
    }

    pub fn get_cli_token(&self) -> Option<CliToken> {
        self.cli_token.read().clone()
    }

    pub fn set_cli_token(&self, token: CliToken) {
        *self.cli_token.write() = Some(token);
    }

    pub fn get_i18n_manager(&self) -> Arc<I18nManager> {
        self.i18n_manager
            .read()
            .as_ref()
            .cloned()
            .expect("i18n manager not initialized")
    }

    pub fn set_i18n_manager(&self, i18n_manager: Arc<I18nManager>) {
        *self.i18n_manager.write() = Some(i18n_manager);
    }

    /// 获取模型管理器。
    pub fn get_model_manager(&self) -> Arc<ModelManager> {
        self.model_manager.clone()
    }

    /// 获取版本更新检查器（共享实例，读上次结果或触发新检查）。
    pub fn get_update_checker(&self) -> Arc<UpdateChecker> {
        self.update_checker.clone()
    }

    /// 收集「组件 id → 触发方式文本」：触发词与触发说明都取自插件元数据。
    ///
    /// 键用 `Configurable::component_id()`：说明书的组件条目来自 `ConfigManager` 的注册组件，
    /// 两处必须同一身份——插件 id 与组件 id 可以不同，用 `PluginMetadata.id` 当键会让触发段对不上。
    /// 内置插件的触发说明是 i18n key，按当前语言翻译；第三方 manifest 给的是原文，翻译服务原样返回。
    /// 不过滤插件的启用状态：说明书同样收录停用组件，模型据此可提示用户先启用。
    ///
    /// 返回：仅含有触发信息的组件；说明书据此渲染组件行的触发段。
    pub fn collect_triggers(&self) -> HashMap<String, String> {
        let i18n = self.get_i18n_manager();
        let lang = i18n.current_language();
        let mut triggers = HashMap::new();
        for (plugin, meta) in self
            .session_dispatcher
            .plugin_registry()
            .get_all_with_metadata()
        {
            let description = meta
                .trigger_description
                .as_deref()
                .map(|key| single_line(&i18n.t(&lang, key)))
                .filter(|text| !text.is_empty());
            if let Some(text) =
                trigger_text(description.as_deref(), &meta.trigger_keywords, meta.mode)
            {
                triggers.insert(plugin.component_id().to_string(), text);
            }
        }
        triggers
    }
}

/// 触发文本的字符上限：第三方文本进说明书是逐行渲染，超长会挤占上下文。
const TRIGGER_TEXT_MAX_CHARS: usize = 200;

/// 生成触发段文本：说明优先，关键词按插件形态标注叫法；两者都没有时返回 None。
///
/// 参数：description - 已压成单行、按当前语言翻译好的触发说明；
/// keywords - 插件声明的触发词（去首尾空白、丢弃空串）；mode - 插件形态。
/// 返回：自含标签的触发段文本（如「触发词：fy / tr」），由说明书组件行直接拼接。
fn trigger_text(
    description: Option<&str>,
    keywords: &[String],
    mode: PluginMode,
) -> Option<String> {
    // 「触发词」只对行内插件成立；面板插件的关键词是候选搜索关键字，输入后作为候选项出现
    let keyword_label = if mode == PluginMode::Inline {
        "触发词"
    } else {
        "候选关键词"
    };
    let keywords = keywords
        .iter()
        .map(|keyword| keyword.trim())
        .filter(|keyword| !keyword.is_empty())
        .collect::<Vec<_>>()
        .join(" / ");
    match (description, keywords.is_empty()) {
        (Some(description), false) => Some(format!("{description}（{keyword_label}：{keywords}）")),
        (Some(description), true) => Some(description.to_string()),
        (None, false) => Some(format!("{keyword_label}：{keywords}")),
        (None, true) => None,
    }
}

/// 压成单行并截断：说明书按行渲染，第三方文本里的换行与连续空白会破坏行结构。
fn single_line(text: &str) -> String {
    let joined = text.split_whitespace().collect::<Vec<_>>().join(" ");
    joined.chars().take(TRIGGER_TEXT_MAX_CHARS).collect()
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("waiting_hashmap", &self.waiting_hashmap)
            .finish()
    }
}
