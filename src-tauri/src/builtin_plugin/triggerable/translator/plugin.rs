use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tracing::info;
use zerolaunch_plugin_api::config::{
    ComponentCore, ComponentType, ConfigError, Configurable, SettingDefinition,
};
use zerolaunch_plugin_api::host::PluginHandle;
use zerolaunch_plugin_api::services::IconRequest;
use zerolaunch_plugin_api::{
    PanelInteraction, PanelKeyAction, PanelKeyBinding, PanelQueryTrigger, Plugin, PluginContext,
    PluginError, PluginKind, PluginMetadata, PluginMode, Query, QueryChannel, QueryResponse,
    ResultAction,
};

use crate::core::config::setting_builders::SchemaBuilder;
use crate::plugin_framework::builtin_registry::PluginEntry;

use super::provider::{
    LanguageSupport, SenseEntry, TranslateRequest, TranslationProvider, TranslationResult,
};
use super::providers::HostModelProvider;
use super::query_parser::{parse_search_term, LangCatalog, ParseError, ParsedQuery};

/// 翻译插件 — 解析带 `@语言码` 前缀的查询并调用已启用翻译引擎，将结果渲染为面板。
///
/// 仅在 triggerable 插件管道中使用，由 builtin_registry 注册；
/// 面板复制等动作统一经 execute_action 委托后端执行。
pub struct TranslatorPlugin {
    /// 组件 ID、名称、类型等基础元数据。
    core: ComponentCore,
    /// 插件元数据（id、名称、触发词等）。
    metadata: PluginMetadata,
    /// 翻译设置（内部可变性：apply_settings 时写入，query 时读取）。
    inner: RwLock<TranslatorSettings>,
    /// 最近一次成功翻译的译文文本，供 execute_action 写入剪贴板。
    last_result_text: RwLock<Option<String>>,
    /// PluginHandle（init 时发放），供 execute_action 经句柄访问平台能力。
    handle: RwLock<Option<Arc<PluginHandle>>>,
    /// 宿主模型引擎（init 时注入 handle，apply_settings 时同步 model_id）。
    host_provider: Arc<HostModelProvider>,
}

/// 语言代码 → 展示名称映射。
/// 服务于 schema select 选项的标签展示。
fn language_display_name(code: &str) -> String {
    match code {
        "zh" => "简体中文".into(),
        "en" => "English".into(),
        "ja" => "日本語".into(),
        "ko" => "한국어".into(),
        "fr" => "Français".into(),
        "de" => "Deutsch".into(),
        "es" => "Español".into(),
        "pt" => "Português".into(),
        "ru" => "Русский".into(),
        "ar" => "العربية".into(),
        "th" => "ไทย".into(),
        "vi" => "Tiếng Việt".into(),
        "it" => "Italiano".into(),
        "nl" => "Nederlands".into(),
        "pl" => "Polski".into(),
        "tr" => "Türkçe".into(),
        _ => code.to_string(),
    }
}

/// 翻译插件的持久化设置（Configurable 数据模型）。
///
/// 由 ConfigManager 序列化为 JSON 存储，经 config_get_settings / config_apply_settings
/// 与前端 TranslatorSettings.vue 双向同步；键名使用 snake_case 与前端契约一致。
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TranslatorSettings {
    /// 翻译模式：live（即时翻译）/ on_enter（回车触发）。
    #[serde(rename = "translate_mode", default = "default_translate_mode")]
    translate_mode: String,
    /// 默认目标语言码（无显式 `@目标` 前缀时使用）。
    #[serde(rename = "default_target", default = "default_target")]
    default_target: String,
    /// 单次翻译请求超时（毫秒）。
    #[serde(rename = "request_timeout_ms", default = "default_request_timeout_ms")]
    request_timeout_ms: u64,
    /// 即时翻译模式下的防抖等待时间（秒），减少冗余请求。
    #[serde(rename = "live_debounce_secs", default = "default_live_debounce_secs")]
    live_debounce_secs: f64,
    /// 宿主模型清单中的全局模型 id（如 "openai/gpt-4o-mini"）。
    #[serde(rename = "model_id", default)]
    model_id: String,
}

const TRANSLATE_MODE_LIVE: &str = "live";
const TRANSLATE_MODE_ON_ENTER: &str = "on_enter";

fn default_translate_mode() -> String {
    TRANSLATE_MODE_LIVE.into()
}

fn default_target() -> String {
    "zh".into()
}

fn default_request_timeout_ms() -> u64 {
    15000
}

fn default_live_debounce_secs() -> f64 {
    0.5
}

impl Default for TranslatorSettings {
    fn default() -> Self {
        Self {
            translate_mode: default_translate_mode(),
            default_target: default_target(),
            request_timeout_ms: default_request_timeout_ms(),
            live_debounce_secs: default_live_debounce_secs(),
            model_id: String::new(),
        }
    }
}

impl TranslatorSettings {
    fn is_on_enter_mode(&self) -> bool {
        self.translate_mode == TRANSLATE_MODE_ON_ENTER
    }
}

impl Default for TranslatorPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl TranslatorPlugin {
    pub fn new() -> Self {
        let host_provider = Arc::new(HostModelProvider::new());

        Self {
            core: ComponentCore::new(
                "translator".to_string(),
                t_key!("translator", "name").to_string(),
                t_key!("translator", "description").to_string(),
                ComponentType::Plugin,
                0,
            ),
            metadata: PluginMetadata {
                id: "translator".to_string(),
                // name/description 与 ComponentCore 同用 i18n key（key-or-literal），
                // 消除插件级与组件级元数据的双源硬编码
                name: t_key!("translator", "name").to_string(),
                // 内置插件无独立版本/作者（随应用分发），UI 按内置标识展示
                version: String::new(),
                description: t_key!("translator", "description").to_string(),
                author: String::new(),
                trigger_keywords: vec!["fy".into(), "tr".into(), "翻译".into()],
                supported_os: vec![
                    "windows".to_string(),
                    "macos".to_string(),
                    "linux".to_string(),
                ],
                priority: 90,
                kind: PluginKind::Builtin,
                // 行内插件：仅关键词（fy/tr/翻译）唤醒，无全局热键，不展示图标
                hotkey: None,
                icon: None,
                mode: PluginMode::Inline,
            },
            inner: RwLock::new(TranslatorSettings::default()),
            last_result_text: RwLock::new(None),
            handle: RwLock::new(None),
            host_provider,
        }
    }

    /// 同步宿主模型引擎的运行时配置（init 注入 handle，apply_settings 同步 model_id）。
    fn sync_host_model(&self, settings: &TranslatorSettings) {
        self.host_provider.set_model_id(&settings.model_id);
    }

    /// 宿主模型引擎的语言能力（静态双语列表）。
    fn active_language_support(&self) -> LanguageSupport {
        self.host_provider.language_support()
    }

    fn lang_catalog(&self) -> LangCatalog {
        let support = self.active_language_support();
        LangCatalog::from_codes(support.sources.iter().chain(support.targets.iter()))
    }

    fn usage_message() -> &'static str {
        "用法: fy hello | fy @en 你好 | fy @zh @en hello"
    }

    fn empty_panel(message: &str) -> QueryResponse {
        QueryResponse::CustomPanel {
            panel_type: "translator".to_string(),
            data: json!({
                "query": null,
                "primary": null,
                "alternatives": [],
                "status": "empty",
                "message": message,
            }),
            actions: vec![],
            keep_search_bar: true,
        }
    }

    fn ready_panel(parsed: &ParsedQuery) -> QueryResponse {
        QueryResponse::CustomPanel {
            panel_type: "translator".to_string(),
            data: json!({
                "query": Self::query_to_json(parsed),
                "primary": null,
                "alternatives": [],
                "status": "ready",
                "message": "按 Enter 翻译",
            }),
            actions: vec![],
            keep_search_bar: true,
        }
    }

    fn error_panel(message: String, query: Option<&ParsedQuery>) -> QueryResponse {
        let query_json = query.map(Self::query_to_json).unwrap_or(json!(null));
        QueryResponse::CustomPanel {
            panel_type: "translator".to_string(),
            data: json!({
                "query": query_json,
                "primary": null,
                "alternatives": [],
                "status": "error",
                "message": message,
            }),
            actions: vec![],
            keep_search_bar: true,
        }
    }

    fn query_to_json(q: &ParsedQuery) -> serde_json::Value {
        json!({
            "text": q.text,
            "source": q.source,
            "target": q.target,
            "raw": q.raw,
        })
    }

    fn sense_to_json(s: &SenseEntry) -> serde_json::Value {
        json!({
            "text": s.text,
            "label": s.label,
        })
    }

    fn result_to_json(r: &TranslationResult) -> serde_json::Value {
        let more_senses: Vec<serde_json::Value> =
            r.more_senses.iter().map(Self::sense_to_json).collect();
        json!({
            "providerId": r.provider_id,
            "providerName": r.provider_name,
            "text": r.text,
            "phonetic": r.phonetic,
            "computerSense": r.computer_sense,
            "moreSenses": more_senses,
            "detectedSource": r.detected_source,
            "error": r.error,
        })
    }

    fn result_to_panel(parsed: &ParsedQuery, result: TranslationResult) -> QueryResponse {
        let is_success = result.is_success();
        let primary_json = is_success
            .then(|| Self::result_to_json(&result))
            .unwrap_or(json!(null));

        let message = if is_success {
            json!(null)
        } else {
            json!(result
                .error
                .unwrap_or_else(|| "翻译失败，请稍后重试".into()))
        };

        let actions = if is_success {
            vec![ResultAction {
                id: "copy_primary".to_string(),
                label: "复制译文".to_string(),
                icon: IconRequest::Path("copy".to_string()),
                is_default: true,
                shortcut_key: "Enter".to_string(),
            }]
        } else {
            vec![]
        };

        QueryResponse::CustomPanel {
            panel_type: "translator".to_string(),
            data: json!({
                "query": Self::query_to_json(parsed),
                "primary": primary_json,
                "alternatives": [],
                "status": if is_success { "ok" } else { "error" },
                "message": message,
            }),
            actions,
            keep_search_bar: true,
        }
    }
}

#[async_trait]
impl Configurable for TranslatorPlugin {
    fn core(&self) -> &ComponentCore {
        &self.core
    }

    fn setting_schema(&self) -> Vec<SettingDefinition> {
        let targets = self.active_language_support().targets;
        let lang_options: Vec<(String, String)> = if targets.is_empty() {
            vec![
                ("zh".into(), "简体中文".into()),
                ("en".into(), "English".into()),
            ]
        } else {
            targets
                .iter()
                .map(|s| (s.clone(), language_display_name(s)))
                .collect()
        };

        let lang_refs: Vec<(&str, &str)> = lang_options
            .iter()
            .map(|(v, l)| (v.as_str(), l.as_str()))
            .collect();

        vec![
            SchemaBuilder::select(
                "translate_mode",
                t_key!("translator", "fields.translate_mode.label"),
                t_key!("translator", "fields.translate_mode.desc"),
            )
            .options(&[TRANSLATE_MODE_LIVE, TRANSLATE_MODE_ON_ENTER])
            .group(t_key!("translator", "groups.basic"))
            .order(0)
            .default(TRANSLATE_MODE_LIVE)
            .build(),
            SchemaBuilder::select(
                "default_target",
                t_key!("translator", "fields.default_target.label"),
                t_key!("translator", "fields.default_target.desc"),
            )
            .options_with_labels(&lang_refs)
            .group(t_key!("translator", "groups.basic"))
            .order(1)
            .default("zh")
            .build(),
            SchemaBuilder::number(
                "request_timeout_ms",
                t_key!("translator", "fields.request_timeout_ms.label"),
                t_key!("translator", "fields.request_timeout_ms.desc"),
            )
            .min(1000.0)
            .max(60000.0)
            .step(500.0)
            .group(t_key!("translator", "groups.engine"))
            .order(2)
            .default(15000.0)
            .build(),
            SchemaBuilder::number(
                "live_debounce_secs",
                t_key!("translator", "fields.live_debounce_secs.label"),
                t_key!("translator", "fields.live_debounce_secs.desc"),
            )
            .min(0.1)
            .max(5.0)
            .step(0.1)
            .group(t_key!("translator", "groups.basic"))
            .order(2)
            .default(0.5)
            .build(),
            SchemaBuilder::text(
                "model_id",
                t_key!("translator", "fields.model_id.label"),
                t_key!("translator", "fields.model_id.desc"),
            )
            .group(t_key!("translator", "groups.engine"))
            .order(3)
            .default("")
            .build(),
        ]
    }

    fn get_settings(&self) -> serde_json::Value {
        serde_json::to_value(self.inner.read().clone()).unwrap_or_default()
    }

    async fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        // 解析失败返回错误拒绝保存：unwrap_or_default 会整体静默重置用户配置
        //（含已配 model_id）。逐字段缺失已由 TranslatorSettings 的 serde(default) 兜底。
        let parsed = serde_json::from_value::<TranslatorSettings>(settings)
            .map_err(|e| ConfigError::ValidationFailed(format!("翻译设置解析失败: {e}")))?;
        self.sync_host_model(&parsed);
        *self.inner.write() = parsed;
        Ok(())
    }

    fn get_default_settings(&self) -> serde_json::Value {
        serde_json::to_value(TranslatorSettings::default()).unwrap_or_default()
    }

    fn default_enabled(&self) -> bool {
        true
    }

    fn config_actions(&self) -> Vec<zerolaunch_plugin_api::config::ConfigActionDef> {
        vec![zerolaunch_plugin_api::config::ConfigActionDef {
            action: "list_models".to_string(),
            label: "获取宿主模型列表".to_string(),
            description: "拉取宿主已配置的模型清单（经插件句柄访问宿主模型服务）".to_string(),
        }]
    }

    async fn execute_config_action(
        &self,
        action: &str,
        _params: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        match action {
            "list_models" => {
                let handle = self
                    .handle
                    .read()
                    .clone()
                    .ok_or_else(|| "插件服务句柄不可用".to_string())?;
                let models = handle.model_list();
                serde_json::to_value(models).map_err(|e| format!("序列化模型列表失败: {e}"))
            }
            _ => Err(format!("Unknown config action: {action}")),
        }
    }
}

#[async_trait]
impl Plugin for TranslatorPlugin {
    fn interaction_policy(&self) -> PanelInteraction {
        let settings = self.inner.read();
        if settings.is_on_enter_mode() {
            PanelInteraction {
                query_trigger: PanelQueryTrigger::OnEnter,
                query_debounce_ms: 0,
                // 面板按键契约（声明即接管，状态转换经显式动作触发）：
                // - Enter：Confirm —— 面板已有可执行动作（翻译成功）时执行默认动作（复制译文），
                //   否则发起确认查询（翻译或失败后重试）（宿主 confirmQuery 三分支语义）；
                // - Escape：GoBack —— 返回默认面板（退出翻译面板）；
                // - Ctrl+Enter：直接复制译文（走既有 copy_primary 动作，不触发翻译）。
                bindings: vec![
                    PanelKeyBinding {
                        key: "Enter".to_string(),
                        action: PanelKeyAction::Confirm,
                    },
                    PanelKeyBinding {
                        key: "Escape".to_string(),
                        action: PanelKeyAction::GoBack,
                    },
                    PanelKeyBinding {
                        key: "Ctrl+Enter".to_string(),
                        action: PanelKeyAction::ExecuteAction {
                            action_id: Some("copy_primary".to_string()),
                        },
                    },
                ],
            }
        } else {
            PanelInteraction {
                query_trigger: PanelQueryTrigger::OnInput,
                query_debounce_ms: (settings.live_debounce_secs * 1000.0) as u64,
                // live 模式按键契约（翻译由输入防抖自动触发，Enter 不再承担触发翻译的角色）：
                // - Enter：Confirm —— 面板已有可执行动作（翻译成功）时执行默认动作（复制译文）；
                //   否则（在途/失败/空）由宿主 confirmQuery 裁决（在途防重 no-op、失败重试）；
                // - Ctrl+Enter：直接复制当前已有译文（走 copy_primary，不触发新翻译）；
                // - Escape：GoBack —— 返回默认面板（退出翻译面板）。
                bindings: vec![
                    PanelKeyBinding {
                        key: "Enter".to_string(),
                        action: PanelKeyAction::Confirm,
                    },
                    PanelKeyBinding {
                        key: "Ctrl+Enter".to_string(),
                        action: PanelKeyAction::ExecuteAction {
                            action_id: Some("copy_primary".to_string()),
                        },
                    },
                    PanelKeyBinding {
                        key: "Escape".to_string(),
                        action: PanelKeyAction::GoBack,
                    },
                ],
            }
        }
    }

    async fn init(
        &self,
        _ctx: &PluginContext,
        handle: Option<Arc<PluginHandle>>,
    ) -> Result<(), PluginError> {
        // 保存服务句柄，供 execute_action 经 PluginHandle 访问平台能力（如剪贴板），
        // 并注入宿主模型引擎（model_chat 能力）。
        if let Some(handle) = handle.clone() {
            self.host_provider.set_handle(handle);
        }
        *self.handle.write() = handle;
        let settings = self.inner.read().clone();
        self.sync_host_model(&settings);
        Ok(())
    }

    async fn query(
        &self,
        ctx: &PluginContext,
        query: &Query,
    ) -> Result<QueryResponse, PluginError> {
        let search_term = query.search_term.trim();
        if search_term.is_empty() {
            return Ok(Self::empty_panel(Self::usage_message()));
        }

        let settings = self.inner.read().clone();
        self.sync_host_model(&settings);
        let catalog = self.lang_catalog();
        if catalog.is_empty() {
            return Ok(Self::error_panel(
                "没有可用的翻译引擎或语言能力，请检查启用的引擎设置".into(),
                None,
            ));
        }

        let parsed = match parse_search_term(search_term, &settings.default_target, &catalog) {
            Ok(p) => p,
            Err(ParseError::EmptyText) => {
                return Ok(Self::empty_panel(Self::usage_message()));
            }
            Err(ParseError::InvalidLanguageCode(code)) => {
                return Ok(Self::error_panel(
                    format!("当前引擎不支持语言代码: {}", code),
                    None,
                ));
            }
        };

        let support = self.active_language_support();
        if !support.supports_pair(&parsed.source, &parsed.target) {
            return Ok(Self::error_panel(
                format!(
                    "当前启用引擎不支持语言对 {}→{}",
                    parsed.source, parsed.target
                ),
                Some(&parsed),
            ));
        }

        // 手动模式（onEnter）：非确认查询（输入/路由触发）只返回 ready 提示；
        // 确认查询（用户按 Enter，Query.confirm=true）走翻译路径。
        // 重复 Enter 拦截由前端实现（确认查询在途/同文本已确认时不发查询），后端无跨查询状态。
        if settings.is_on_enter_mode() && !query.confirm {
            return Ok(Self::ready_panel(&parsed));
        }

        let req = TranslateRequest {
            text: parsed.text.clone(),
            source: parsed.source.clone(),
            target: parsed.target.clone(),
        };

        // 单引擎直调宿主模型：外层超时保护（与旧 registry 语义一致）。
        let result = match tokio::time::timeout(
            std::time::Duration::from_millis(settings.request_timeout_ms),
            self.host_provider.translate(&req),
        )
        .await
        {
            Ok(r) => r,
            Err(_) => TranslationResult::err(
                super::providers::PROVIDER_ID,
                "宿主模型",
                "翻译超时，请重试",
            ),
        };

        // 缓存译文文本，供 execute_action 写入剪贴板。
        // 仅 GUI 通道且查询仍最新时写入：CLI/调试查询为只读辅助路径，
        // 不得改写 GUI 剪贴板缓存（execute_action 无通道区分，
        // 复制动作必须始终拿到与 GUI 面板一致的译文）。
        if ctx.is_query_current() && ctx.query_channel == QueryChannel::Ui {
            *self.last_result_text.write() = result.is_success().then(|| result.text.clone());
        } else {
            info!(
                trace_id = %ctx.trace_id,
                query_revision = ctx.query_revision(),
                site = "plugin_cache",
                "查询过期，丢弃翻译结果缓存写入"
            );
        }

        Ok(Self::result_to_panel(&parsed, result))
    }

    async fn execute_action(
        &self,
        _ctx: &PluginContext,
        action_id: &str,
        _payload: serde_json::Value,
    ) -> Result<(), PluginError> {
        if action_id == "copy_primary" || action_id.starts_with("copy_alt:") {
            let text = self.last_result_text.read().clone();
            if let Some(text) = text {
                // 经 PluginHandle 访问剪贴板能力（init 时发放）。
                let handle =
                    self.handle.read().clone().ok_or_else(|| {
                        PluginError::ActionFailed("插件服务句柄不可用".to_string())
                    })?;
                handle
                    .set_clipboard_text(&text)
                    .map_err(|e| PluginError::ActionFailed(format!("剪贴板写入失败: {}", e)))?;
            }
            Ok(())
        } else {
            Err(PluginError::ActionFailed(format!(
                "未知动作: {}",
                action_id
            )))
        }
    }
}

fn build_translator_plugin() -> (Arc<dyn Configurable>, Arc<dyn Plugin>, PluginMetadata) {
    let plugin = Arc::new(TranslatorPlugin::new());
    let metadata = plugin.metadata.clone();
    let configurable: Arc<dyn Configurable> = plugin.clone();
    let plugin: Arc<dyn Plugin> = plugin;
    (configurable, plugin, metadata)
}

::inventory::submit! {
    PluginEntry {
        component_id: "translator",
        priority: 10,
        factory: build_translator_plugin,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_query(search_term: &str) -> Query {
        sample_query_with_confirm(search_term, false)
    }

    /// 构造带确认标志的查询：confirm=true 模拟用户按 Enter 触发。
    fn sample_query_with_confirm(search_term: &str, confirm: bool) -> Query {
        Query {
            id: "1".into(),
            raw_query: format!("fy {search_term}"),
            search_term: search_term.into(),
            confirm,
        }
    }

    async fn apply_on_enter(plugin: &TranslatorPlugin) {
        plugin
            .apply_settings(json!({
                "translate_mode": TRANSLATE_MODE_ON_ENTER,
                "default_target": "zh",
                "request_timeout_ms": 15000,
                "model_id": "",
            }))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn query_without_model_returns_error_panel() {
        let plugin = TranslatorPlugin::new();
        let ctx = PluginContext::new("test");
        let resp = plugin.query(&ctx, &sample_query("hello")).await.unwrap();

        let policy = plugin.interaction_policy();
        match resp {
            QueryResponse::CustomPanel {
                panel_type, data, ..
            } => {
                assert_eq!(panel_type, "translator");
                assert_eq!(data["status"], "error");
                assert_eq!(policy.query_trigger, PanelQueryTrigger::OnInput);
                assert_eq!(policy.query_debounce_ms, 500);
                let msg = data["message"].as_str().unwrap_or("");
                assert!(
                    msg.contains("选择翻译模型") || msg.contains("句柄"),
                    "期望未选模型相关错误提示，实际: {msg}"
                );
            }
            other => panic!("期望 CustomPanel，实际 {:?}", other),
        }
    }

    #[tokio::test]
    async fn query_empty_returns_empty_status() {
        let plugin = TranslatorPlugin::new();
        let ctx = PluginContext::new("test");
        let q = Query {
            id: "2".into(),
            raw_query: "fy".into(),
            search_term: "".into(),
            confirm: false,
        };
        let resp = plugin.query(&ctx, &q).await.unwrap();

        let policy = plugin.interaction_policy();
        match resp {
            QueryResponse::CustomPanel { data, actions, .. } => {
                assert_eq!(data["status"], "empty");
                assert!(actions.is_empty());
                assert_eq!(policy.query_trigger, PanelQueryTrigger::OnInput);
                assert_eq!(policy.query_debounce_ms, 500);
            }
            other => panic!("期望 CustomPanel，实际 {:?}", other),
        }
    }

    #[tokio::test]
    async fn query_invalid_lang_returns_error() {
        let plugin = TranslatorPlugin::new();
        let ctx = PluginContext::new("test");
        let resp = plugin
            .query(&ctx, &sample_query("@xx hello"))
            .await
            .unwrap();

        match resp {
            QueryResponse::CustomPanel { data, .. } => {
                assert_eq!(data["status"], "error");
            }
            other => panic!("期望 CustomPanel，实际 {:?}", other),
        }
    }

    #[tokio::test]
    async fn on_enter_first_query_returns_ready() {
        let plugin = TranslatorPlugin::new();
        apply_on_enter(&plugin).await;

        let ctx = PluginContext::new("test");
        let resp = plugin.query(&ctx, &sample_query("hello")).await.unwrap();

        let policy = plugin.interaction_policy();
        match resp {
            QueryResponse::CustomPanel { data, actions, .. } => {
                assert_eq!(data["status"], "ready");
                assert_eq!(data["query"]["text"], "hello");
                assert_eq!(data["message"], "按 Enter 翻译");
                assert!(actions.is_empty());
                assert_eq!(policy.query_trigger, PanelQueryTrigger::OnEnter);
                assert_eq!(policy.query_debounce_ms, 0);
            }
            other => panic!("期望 CustomPanel，实际 {:?}", other),
        }
    }

    #[tokio::test]
    async fn on_enter_confirm_query_enters_translate_path() {
        let plugin = TranslatorPlugin::new();
        apply_on_enter(&plugin).await;

        let ctx = PluginContext::new("test");
        // 非确认查询（输入/路由触发）→ ready
        let first = plugin.query(&ctx, &sample_query("hello")).await.unwrap();
        let policy = plugin.interaction_policy();
        match &first {
            QueryResponse::CustomPanel { data, .. } => {
                assert_eq!(data["status"], "ready");
                assert_eq!(policy.query_trigger, PanelQueryTrigger::OnEnter);
                assert_eq!(policy.query_debounce_ms, 0);
            }
            other => panic!("首次应 ready，实际 {:?}", other),
        }

        // 确认查询（Enter 触发，confirm=true）→ 翻译路径（未选模型 → error）
        let second = plugin
            .query(&ctx, &sample_query_with_confirm("hello", true))
            .await
            .unwrap();
        match second {
            QueryResponse::CustomPanel { data, .. } => {
                assert_eq!(data["status"], "error");
                let msg = data["message"].as_str().unwrap_or("");
                assert!(
                    msg.contains("选择翻译模型") || msg.contains("句柄"),
                    "期望未选模型错误，实际: {msg}"
                );
                assert_eq!(policy.query_trigger, PanelQueryTrigger::OnEnter);
                assert_eq!(policy.query_debounce_ms, 0);
            }
            other => panic!("期望 CustomPanel，实际 {:?}", other),
        }
    }

    #[tokio::test]
    async fn on_enter_edit_then_confirm_translates_directly() {
        let plugin = TranslatorPlugin::new();
        apply_on_enter(&plugin).await;
        let ctx = PluginContext::new("test");

        // 面板内改文本后非确认查询 → ready（展示最新文本）
        let resp = plugin.query(&ctx, &sample_query("world")).await.unwrap();
        match &resp {
            QueryResponse::CustomPanel { data, .. } => {
                assert_eq!(data["status"], "ready");
                assert_eq!(data["query"]["text"], "world");
            }
            other => panic!("期望 ready，实际 {:?}", other),
        }

        // 随后确认（Enter）→ 直接翻译路径，与文本改动历史无关（不再需要二次 Enter）
        let confirm = plugin
            .query(&ctx, &sample_query_with_confirm("world", true))
            .await
            .unwrap();
        match confirm {
            QueryResponse::CustomPanel { data, .. } => {
                assert_eq!(data["status"], "error");
            }
            other => panic!("期望 CustomPanel，实际 {:?}", other),
        }
    }
}
