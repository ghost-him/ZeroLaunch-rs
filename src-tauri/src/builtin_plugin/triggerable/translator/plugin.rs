use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use zerolaunch_plugin_api::config::{
    ComponentCore, ComponentType, ConfigError, Configurable, PrimitiveType, SettingDefinition,
};
use zerolaunch_plugin_api::host::PluginHandle;
use zerolaunch_plugin_api::services::IconRequest;
use zerolaunch_plugin_api::{
    Plugin, PluginContext, PluginError, PluginMetadata, Query, QueryResponse, ResultAction,
};

use crate::core::config::setting_builders::SchemaBuilder;
use crate::plugin_framework::builtin_registry::PluginEntry;

use super::provider::{LanguageSupport, SenseEntry, TranslateRequest, TranslationResult};
use super::providers::{
    LlmConfig, MockProvider, OpenAiCompatibleProvider, MOCK_PROVIDER_ID, MOCK_PROVIDER_NAME,
    PROVIDER_ID,
};
use super::query_parser::{parse_search_term, LangCatalog, ParseError, ParsedQuery};
use super::registry::{AggregateResult, AggregateStatus, ProviderRegistry};

pub struct TranslatorPlugin {
    core: ComponentCore,
    metadata: PluginMetadata,
    inner: RwLock<TranslatorSettings>,
    llm_config: Arc<RwLock<LlmConfig>>,
    registry: ProviderRegistry,
    /// `on_enter` 模式：同一正文连续第二次 query 才真正请求 LLM。
    on_enter_gate: RwLock<OnEnterGate>,
}

/// 手动确认门控状态（仅插件内部，不扩展框架 Query）。
#[derive(Default)]
struct OnEnterGate {
    /// 已展示 ready、等待 Enter 再查的正文指纹
    pending: Option<String>,
    /// 上次已完成翻译的正文指纹 + 面板（同文再次非确认查询时复用，避免结果被 ready 盖掉）
    last_done: Option<String>,
    last_panel: Option<QueryResponse>,
}

fn search_fingerprint(search_term: &str) -> String {
    search_term
        .trim()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

enum OnEnterDecision {
    Ready,
    Commit,
    Reuse(QueryResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TranslatorSettings {
    #[serde(default = "default_enabled_true")]
    enabled: bool,
    #[serde(default = "default_translate_mode")]
    translate_mode: String,
    #[serde(default = "default_target")]
    default_target: String,
    /// 参与并行翻译的引擎 id 列表；列表顺序即展示优先顺序（首个成功结果作为主结果）。
    #[serde(default = "default_enabled_providers")]
    enabled_providers: Vec<String>,
    #[serde(default = "default_request_timeout_ms")]
    request_timeout_ms: u64,
    /// 厂商预设；选非「自定义」时在 normalize 中写入对应 Base URL。
    #[serde(default = "default_llm_vendor")]
    llm_vendor: String,
    #[serde(default)]
    llm_base_url: String,
    #[serde(default)]
    llm_api_key: String,
    #[serde(default)]
    llm_model: String,
}

const TRANSLATE_MODE_LIVE: &str = "live";
const TRANSLATE_MODE_ON_ENTER: &str = "on_enter";
const MODE_LIVE_LABEL: &str = "即时翻译";
const MODE_ON_ENTER_LABEL: &str = "按 Enter 翻译";

const LLM_VENDOR_CUSTOM: &str = "自定义";
const LLM_VENDOR_OPTIONS: &[&str] = &[
    "DeepSeek",
    "智谱 GLM",
    "OpenAI",
    "硅基流动",
    "阿里云百炼",
    "腾讯云 TokenHub",
    "Kimi",
    "小米 MiMo",
    LLM_VENDOR_CUSTOM,
];

const PROVIDER_LABEL_OPENAI: &str = "OpenAI 兼容";

fn default_enabled_true() -> bool {
    true
}

fn default_translate_mode() -> String {
    MODE_LIVE_LABEL.into()
}

fn default_target() -> String {
    "zh".into()
}

fn default_enabled_providers() -> Vec<String> {
    vec![PROVIDER_ID.into()]
}

fn default_request_timeout_ms() -> u64 {
    15000
}

fn default_llm_vendor() -> String {
    LLM_VENDOR_CUSTOM.into()
}

fn vendor_base_url(vendor: &str) -> Option<&'static str> {
    match vendor {
        "DeepSeek" => Some("https://api.deepseek.com"),
        "智谱 GLM" => Some("https://open.bigmodel.cn/api/paas/v4"),
        "OpenAI" => Some("https://api.openai.com/v1"),
        "硅基流动" => Some("https://api.siliconflow.cn/v1"),
        "阿里云百炼" => Some("https://dashscope.aliyuncs.com/compatible-mode/v1"),
        "腾讯云 TokenHub" => Some("https://tokenhub.tencentmaas.com/v1"),
        "Kimi" => Some("https://api.moonshot.cn/v1"),
        "小米 MiMo" => Some("https://api.xiaomimimo.com/v1"),
        _ => None,
    }
}

fn language_option_label(code: &str) -> String {
    let name = match code {
        "zh" => "简体中文",
        "zh-TR" => "繁体中文",
        "yue" => "粤语",
        "en" => "英语",
        "fr" => "法语",
        "pt" => "葡萄牙语",
        "es" => "西班牙语",
        "ja" => "日语",
        "tr" => "土耳其语",
        "ru" => "俄语",
        "ar" => "阿拉伯语",
        "ko" => "韩语",
        "th" => "泰语",
        "it" => "意大利语",
        "de" => "德语",
        "vi" => "越南语",
        "ms" => "马来语",
        "id" => "印尼语",
        other => other,
    };
    if name == code {
        code.to_string()
    } else {
        format!("{name} ({code})")
    }
}

fn language_code_from_option(opt: &str) -> String {
    let t = opt.trim();
    if let Some(start) = t.rfind('(') {
        if let Some(end) = t.rfind(')') {
            if end > start + 1 {
                return t[start + 1..end].trim().to_string();
            }
        }
    }
    t.to_string()
}

fn mode_to_label(mode: &str) -> String {
    if mode == MODE_ON_ENTER_LABEL || mode == TRANSLATE_MODE_ON_ENTER {
        MODE_ON_ENTER_LABEL.into()
    } else {
        // 兼容旧值 "live" 与中文标签
        let _ = TRANSLATE_MODE_LIVE;
        MODE_LIVE_LABEL.into()
    }
}

fn provider_label(id: &str) -> &str {
    if id == PROVIDER_ID || id == "openai-compatible" {
        PROVIDER_LABEL_OPENAI
    } else if id == MOCK_PROVIDER_ID || id == "mock" {
        MOCK_PROVIDER_NAME
    } else {
        id
    }
}

fn provider_id_from_label(label: &str) -> String {
    if label == PROVIDER_LABEL_OPENAI || label == "openai-compatible" {
        PROVIDER_ID.into()
    } else if label == MOCK_PROVIDER_NAME || label == "mock" {
        MOCK_PROVIDER_ID.into()
    } else {
        label.to_string()
    }
}

impl Default for TranslatorSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            translate_mode: default_translate_mode(),
            default_target: default_target(),
            enabled_providers: default_enabled_providers(),
            request_timeout_ms: default_request_timeout_ms(),
            llm_vendor: default_llm_vendor(),
            llm_base_url: String::new(),
            llm_api_key: String::new(),
            llm_model: String::new(),
        }
    }
}

impl TranslatorSettings {
    /// 规范化：引擎 id、触发模式、语言码；非自定义厂商写入 Base URL。
    fn normalize(mut self) -> Self {
        self.translate_mode = mode_to_label(&self.translate_mode);
        self.default_target = language_code_from_option(&self.default_target);
        self.enabled_providers = self
            .enabled_providers
            .iter()
            .map(|p| provider_id_from_label(p))
            .filter(|p| !p.is_empty())
            .collect();
        if self.enabled_providers.is_empty() {
            self.enabled_providers = default_enabled_providers();
        }
        if !LLM_VENDOR_OPTIONS.contains(&self.llm_vendor.as_str()) {
            self.llm_vendor = default_llm_vendor();
        }
        if let Some(url) = vendor_base_url(&self.llm_vendor) {
            self.llm_base_url = url.to_string();
        }
        self
    }

    fn preferred_provider_id(&self) -> &str {
        self.enabled_providers
            .first()
            .map(|s| s.as_str())
            .unwrap_or(PROVIDER_ID)
    }

    fn is_on_enter_mode(&self) -> bool {
        self.translate_mode == MODE_ON_ENTER_LABEL
            || self.translate_mode == TRANSLATE_MODE_ON_ENTER
    }

    /// 供 DynamicForm 展示的中文选项视图（不改动内存中的规范存储前请先 clone）。
    fn for_ui_display(mut self) -> Self {
        self.translate_mode = mode_to_label(&self.translate_mode);
        self.default_target = language_option_label(&self.default_target);
        self.enabled_providers = self
            .enabled_providers
            .iter()
            .map(|id| provider_label(id).to_string())
            .collect();
        self
    }
}

impl Default for TranslatorPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl TranslatorPlugin {
    pub fn new() -> Self {
        let llm_config = Arc::new(RwLock::new(LlmConfig::default()));
        let registry = ProviderRegistry::new(vec![
            Arc::new(OpenAiCompatibleProvider::new(Arc::clone(&llm_config))),
            Arc::new(MockProvider),
        ]);

        Self {
            core: ComponentCore::new(
                "translator".to_string(),
                "翻译".to_string(),
                "在搜索栏中进行多引擎翻译".to_string(),
                ComponentType::Plugin,
                0,
            ),
            metadata: PluginMetadata {
                id: "translator".to_string(),
                name: "翻译".to_string(),
                version: "1.0.0".to_string(),
                description: "多引擎翻译插件".to_string(),
                author: "ZeroLaunch".to_string(),
                trigger_keywords: vec!["fy".into(), "tr".into(), "翻译".into()],
                supported_os: vec![
                    "windows".to_string(),
                    "macos".to_string(),
                    "linux".to_string(),
                ],
                priority: 90,
            },
            inner: RwLock::new(TranslatorSettings::default()),
            llm_config,
            registry,
            on_enter_gate: RwLock::new(OnEnterGate::default()),
        }
    }

    fn clear_on_enter_gate(&self) {
        *self.on_enter_gate.write() = OnEnterGate::default();
    }

    /// `on_enter` 下：首次同文 → Ready；连续第二次 → Commit；已完成同文 → 复用面板。
    fn on_enter_decision(&self, fingerprint: &str) -> OnEnterDecision {
        let mut gate = self.on_enter_gate.write();
        if gate.pending.as_deref() == Some(fingerprint) {
            gate.pending = None;
            return OnEnterDecision::Commit;
        }
        if gate.last_done.as_deref() == Some(fingerprint) {
            if let Some(panel) = gate.last_panel.clone() {
                return OnEnterDecision::Reuse(panel);
            }
        }
        gate.pending = Some(fingerprint.to_string());
        gate.last_done = None;
        gate.last_panel = None;
        OnEnterDecision::Ready
    }

    fn remember_on_enter_result(&self, fingerprint: &str, panel: &QueryResponse) {
        let mut gate = self.on_enter_gate.write();
        gate.pending = None;
        gate.last_done = Some(fingerprint.to_string());
        gate.last_panel = Some(panel.clone());
    }

    fn sync_llm_config(&self, settings: &TranslatorSettings) {
        *self.llm_config.write() = LlmConfig {
            base_url: settings.llm_base_url.clone(),
            api_key: settings.llm_api_key.clone(),
            model: settings.llm_model.clone(),
        };
    }

    /// 当前启用引擎在运行时配置下的语言并集。
    fn active_language_support(&self, settings: &TranslatorSettings) -> LanguageSupport {
        self.registry
            .language_support_for(&settings.enabled_providers)
    }

    fn lang_catalog(&self, settings: &TranslatorSettings) -> LangCatalog {
        let support = self.active_language_support(settings);
        LangCatalog::from_codes(support.sources.iter().chain(support.targets.iter()))
    }

    fn usage_message() -> &'static str {
        "用法: fy hello | fy en 你好 | fy zh en hello"
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

    fn status_str(status: &AggregateStatus) -> &'static str {
        match status {
            AggregateStatus::Ok => "ok",
            AggregateStatus::Partial => "partial",
            AggregateStatus::Error => "error",
        }
    }

    fn aggregate_to_panel(parsed: &ParsedQuery, agg: AggregateResult) -> QueryResponse {
        let has_primary = agg
            .primary
            .as_ref()
            .map(|p| p.is_success())
            .unwrap_or(false);

        let primary_json = agg
            .primary
            .as_ref()
            .map(Self::result_to_json)
            .unwrap_or(json!(null));

        let alternatives: Vec<serde_json::Value> = agg
            .alternatives
            .iter()
            .map(Self::result_to_json)
            .collect();

        let message = if has_primary {
            json!(null)
        } else {
            let detail = agg
                .primary
                .as_ref()
                .and_then(|p| p.error.clone())
                .or_else(|| {
                    agg.alternatives
                        .iter()
                        .find_map(|a| a.error.clone())
                });
            json!(detail.unwrap_or_else(|| "翻译失败，请稍后重试".into()))
        };

        let actions = if has_primary {
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
                "alternatives": alternatives,
                "status": Self::status_str(&agg.status),
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
        let settings = self.inner.read().clone();
        self.sync_llm_config(&settings);
        let targets = self.active_language_support(&settings).targets;
        let lang_options: Vec<String> = if targets.is_empty() {
            vec![
                language_option_label("zh"),
                language_option_label("en"),
            ]
        } else {
            targets.iter().map(|c| language_option_label(c)).collect()
        };
        let lang_refs: Vec<&str> = lang_options.iter().map(|s| s.as_str()).collect();

        vec![
            SchemaBuilder::select(
                "translate_mode",
                "翻译触发",
                "即时：输入即翻译；按 Enter：确认后才请求，节省 token",
            )
            .options(&[MODE_LIVE_LABEL, MODE_ON_ENTER_LABEL])
            .group("基础")
            .order(0)
            .default(MODE_LIVE_LABEL)
            .build(),
            SchemaBuilder::select(
                "default_target",
                "默认目标语言",
                "未写语言码时的目标语（源语自动检测；若与源语相同则回退到另一常用语）",
            )
            .options(&lang_refs)
            .group("基础")
            .order(1)
            .default(language_option_label("zh"))
            .build(),
            SchemaBuilder::array(
                "enabled_providers",
                "翻译引擎",
                "参与并行翻译的引擎；列表顺序即结果优先顺序",
            )
            .primitive_item(PrimitiveType::Select {
                options: vec![PROVIDER_LABEL_OPENAI.into(), MOCK_PROVIDER_NAME.into()],
            })
            .group("引擎")
            .order(2)
            .default(json!([PROVIDER_LABEL_OPENAI]))
            .build(),
            SchemaBuilder::number("request_timeout_ms", "超时（毫秒）", "单个引擎的请求超时时间")
                .min(1000.0)
                .max(60000.0)
                .step(500.0)
                .group("引擎")
                .order(3)
                .default(15000.0)
                .build(),
            SchemaBuilder::select(
                "llm_vendor",
                "厂商预设",
                "点「应用」后写入对应 Base URL；选「自定义」不改写地址",
            )
            .options(LLM_VENDOR_OPTIONS)
            .group("LLM 服务")
            .order(9)
            .default(LLM_VENDOR_CUSTOM)
            .build(),
            SchemaBuilder::text(
                "llm_base_url",
                "Base URL",
                "OpenAI 兼容 API 根地址（如 https://api.deepseek.com）",
            )
            .group("LLM 服务")
            .order(10)
            .default("")
            .build(),
            SchemaBuilder::text(
                "llm_api_key",
                "API Key",
                "LLM 服务的 API 密钥（请妥善保管）",
            )
            .group("LLM 服务")
            .order(11)
            .default("")
            .build(),
            SchemaBuilder::text(
                "llm_model",
                "Model",
                "模型名称（如 deepseek-chat、moonshot-v1-8k）",
            )
            .group("LLM 服务")
            .order(12)
            .default("")
            .build(),
        ]
    }

    fn get_settings(&self) -> serde_json::Value {
        serde_json::to_value(self.inner.read().clone().for_ui_display()).unwrap_or_default()
    }

    fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        let parsed = serde_json::from_value::<TranslatorSettings>(settings)
            .unwrap_or_default()
            .normalize();
        self.sync_llm_config(&parsed);
        self.clear_on_enter_gate();
        *self.inner.write() = parsed;
        Ok(())
    }

    fn get_default_settings(&self) -> serde_json::Value {
        serde_json::to_value(TranslatorSettings::default().for_ui_display()).unwrap_or_default()
    }

    fn default_enabled(&self) -> bool {
        true
    }
}

#[async_trait]
impl Plugin for TranslatorPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    async fn init(
        &self,
        _ctx: &PluginContext,
        _handle: Arc<PluginHandle>,
    ) -> Result<(), PluginError> {
        let settings = self.inner.read().clone();
        self.sync_llm_config(&settings);
        Ok(())
    }

    async fn query(
        &self,
        _ctx: &PluginContext,
        query: &Query,
    ) -> Result<QueryResponse, PluginError> {
        let search_term = query.search_term.trim();
        if search_term.is_empty() {
            self.clear_on_enter_gate();
            return Ok(Self::empty_panel(Self::usage_message()));
        }

        let settings = self.inner.read().clone();
        self.sync_llm_config(&settings);
        let catalog = self.lang_catalog(&settings);
        if catalog.is_empty() {
            return Ok(Self::error_panel(
                "没有可用的翻译引擎或语言能力，请检查启用的引擎设置".into(),
                None,
            ));
        }

        let parsed = match parse_search_term(search_term, &settings.default_target, &catalog) {
            Ok(p) => p,
            Err(ParseError::EmptyText) => {
                self.clear_on_enter_gate();
                return Ok(Self::empty_panel(Self::usage_message()));
            }
            Err(ParseError::InvalidLanguageCode(code)) => {
                return Ok(Self::error_panel(
                    format!("当前引擎不支持语言代码: {}", code),
                    None,
                ));
            }
        };

        let support = self.active_language_support(&settings);
        if !support.supports_pair(&parsed.source, &parsed.target) {
            return Ok(Self::error_panel(
                format!(
                    "当前启用引擎不支持语言对 {}→{}",
                    parsed.source, parsed.target
                ),
                Some(&parsed),
            ));
        }

        let fingerprint = search_fingerprint(search_term);

        // 手动模式：同一正文首次 → ready；Enter 再查同文 → 真正翻译（不扩展框架协议）。
        if settings.is_on_enter_mode() {
            match self.on_enter_decision(&fingerprint) {
                OnEnterDecision::Ready => {
                    return Ok(Self::ready_panel(&parsed));
                }
                OnEnterDecision::Reuse(panel) => {
                    return Ok(panel);
                }
                OnEnterDecision::Commit => {}
            }
        } else {
            self.clear_on_enter_gate();
        }

        let req = TranslateRequest {
            text: parsed.text.clone(),
            source: parsed.source.clone(),
            target: parsed.target.clone(),
        };

        let agg = self
            .registry
            .translate_all(
                &req,
                &settings.enabled_providers,
                settings.preferred_provider_id(),
                settings.request_timeout_ms,
            )
            .await;

        let panel = Self::aggregate_to_panel(&parsed, agg);
        if settings.is_on_enter_mode() {
            self.remember_on_enter_result(&fingerprint, &panel);
        }
        Ok(panel)
    }

    async fn execute_action(
        &self,
        _ctx: &PluginContext,
        action_id: &str,
        _payload: serde_json::Value,
    ) -> Result<(), PluginError> {
        if action_id == "copy_primary" || action_id.starts_with("copy_alt:") {
            // 剪贴板由前端写入
            Ok(())
        } else {
            Err(PluginError::ActionFailed(format!(
                "未知动作: {}",
                action_id
            )))
        }
    }
}

fn build_translator_plugin() -> (Arc<dyn Configurable>, Arc<dyn Plugin>) {
    let plugin: Arc<dyn Plugin> = Arc::new(TranslatorPlugin::new());
    let configurable: Arc<dyn Configurable> = plugin.clone();
    (configurable, plugin)
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
        Query {
            id: "1".into(),
            raw_query: format!("fy {search_term}"),
            search_term: search_term.into(),
        }
    }

    fn apply_on_enter(plugin: &TranslatorPlugin) {
        plugin
            .apply_settings(json!({
                "translate_mode": MODE_ON_ENTER_LABEL,
                "default_target": "zh",
                "enabled_providers": [PROVIDER_ID],
                "request_timeout_ms": 15000,
                "llm_vendor": LLM_VENDOR_CUSTOM,
                "llm_base_url": "",
                "llm_api_key": "",
                "llm_model": "",
            }))
            .unwrap();
    }

    #[tokio::test]
    async fn query_without_credentials_returns_error_panel() {
        let plugin = TranslatorPlugin::new();
        let ctx = PluginContext::new("test");
        let resp = plugin.query(&ctx, &sample_query("hello")).await.unwrap();

        match resp {
            QueryResponse::CustomPanel {
                panel_type, data, ..
            } => {
                assert_eq!(panel_type, "translator");
                assert_eq!(data["status"], "error");
                let msg = data["message"].as_str().unwrap_or("");
                assert!(
                    msg.contains("设置") || msg.contains("填写"),
                    "期望 LLM 配置相关错误提示，实际: {msg}"
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
        };
        let resp = plugin.query(&ctx, &q).await.unwrap();

        match resp {
            QueryResponse::CustomPanel {
                data, actions, ..
            } => {
                assert_eq!(data["status"], "empty");
                assert!(actions.is_empty());
            }
            other => panic!("期望 CustomPanel，实际 {:?}", other),
        }
    }

    #[tokio::test]
    async fn query_invalid_lang_returns_error() {
        let plugin = TranslatorPlugin::new();
        let ctx = PluginContext::new("test");
        let resp = plugin
            .query(&ctx, &sample_query("xx hello"))
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
        apply_on_enter(&plugin);

        let ctx = PluginContext::new("test");
        let resp = plugin.query(&ctx, &sample_query("hello")).await.unwrap();

        match resp {
            QueryResponse::CustomPanel { data, actions, .. } => {
                assert_eq!(data["status"], "ready");
                assert_eq!(data["query"]["text"], "hello");
                assert_eq!(data["message"], "按 Enter 翻译");
                assert!(actions.is_empty());
            }
            other => panic!("期望 CustomPanel，实际 {:?}", other),
        }
    }

    #[tokio::test]
    async fn on_enter_second_same_query_enters_translate_path() {
        let plugin = TranslatorPlugin::new();
        apply_on_enter(&plugin);

        let ctx = PluginContext::new("test");
        let q = sample_query("hello");
        let first = plugin.query(&ctx, &q).await.unwrap();
        match &first {
            QueryResponse::CustomPanel { data, .. } => assert_eq!(data["status"], "ready"),
            other => panic!("首次应 ready，实际 {:?}", other),
        }

        let second = plugin.query(&ctx, &q).await.unwrap();
        match second {
            QueryResponse::CustomPanel { data, .. } => {
                // 无凭据时应进入翻译路径并返回 error（而非 ready）
                assert_eq!(data["status"], "error");
                let msg = data["message"].as_str().unwrap_or("");
                assert!(
                    msg.contains("设置") || msg.contains("填写"),
                    "期望进入 LLM 路径的配置错误，实际: {msg}"
                );
            }
            other => panic!("期望 CustomPanel，实际 {:?}", other),
        }
    }

    #[tokio::test]
    async fn on_enter_edit_text_resets_to_ready() {
        let plugin = TranslatorPlugin::new();
        apply_on_enter(&plugin);
        let ctx = PluginContext::new("test");

        let _ = plugin.query(&ctx, &sample_query("hello")).await.unwrap();
        let resp = plugin.query(&ctx, &sample_query("world")).await.unwrap();
        match resp {
            QueryResponse::CustomPanel { data, .. } => {
                assert_eq!(data["status"], "ready");
                assert_eq!(data["query"]["text"], "world");
            }
            other => panic!("期望 ready，实际 {:?}", other),
        }
    }

    #[test]
    fn language_option_roundtrip() {
        assert_eq!(language_code_from_option("简体中文 (zh)"), "zh");
        assert_eq!(language_option_label("zh"), "简体中文 (zh)");
    }

    #[test]
    fn kimi_vendor_fills_moonshot_base_url() {
        let settings = TranslatorSettings {
            llm_vendor: "Kimi".into(),
            llm_base_url: String::new(),
            ..TranslatorSettings::default()
        }
        .normalize();
        assert_eq!(settings.llm_base_url, "https://api.moonshot.cn/v1");
    }

    #[test]
    fn custom_vendor_keeps_base_url() {
        let settings = TranslatorSettings {
            llm_vendor: LLM_VENDOR_CUSTOM.into(),
            llm_base_url: "https://example.com/v1".into(),
            ..TranslatorSettings::default()
        }
        .normalize();
        assert_eq!(settings.llm_base_url, "https://example.com/v1");
    }

}
