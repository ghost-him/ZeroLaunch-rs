use async_trait::async_trait;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tracing::info;
use zerolaunch_plugin_api::config::{
    ComponentCore, ComponentType, ConfigError, Configurable, FieldUiMetadata, PrimitiveType,
    SchemaKind, SchemaNode, SettingDefinition,
};
use zerolaunch_plugin_api::host::PluginHandle;
use zerolaunch_plugin_api::services::IconRequest;
use zerolaunch_plugin_api::{
    PanelInteraction, PanelKeyAction, PanelKeyBinding, PanelQueryTrigger, Plugin, PluginContext,
    PluginError, PluginMetadata, Query, QueryChannel, QueryResponse, ResultAction,
};

use crate::core::config::setting_builders::SchemaBuilder;
use crate::plugin_framework::builtin_registry::PluginEntry;
use std::collections::{BTreeMap, BTreeSet};

use super::provider::{LanguageSupport, SenseEntry, TranslateRequest, TranslationResult};
use super::providers::{
    LlmConfig, MockProvider, OpenAiCompatibleProvider, MOCK_PROVIDER_ID, PROVIDER_ID,
};
use super::query_parser::{parse_search_term, LangCatalog, ParseError, ParsedQuery};
use super::registry::{AggregateResult, AggregateStatus, ProviderRegistry};

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
    /// LLM 引擎连接配置（内部可变性：apply_settings 同步 llm_* 字段时写入）。
    llm_config: Arc<RwLock<LlmConfig>>,
    /// 已注册翻译引擎的聚合注册表，负责语言能力汇总与并行翻译分发。
    registry: ProviderRegistry,
    /// 最近一次成功翻译的译文文本，供 execute_action 写入剪贴板。
    last_result_text: RwLock<Option<String>>,
    /// PluginHandle（init 时发放），供 execute_action 经句柄访问平台能力。
    handle: RwLock<Option<Arc<PluginHandle>>>,
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
    /// 参与并行翻译的引擎 id 列表；列表顺序即展示优先顺序（首个成功结果作为主结果）。
    #[serde(rename = "enabled_providers", default = "default_enabled_providers")]
    enabled_providers: Vec<String>,
    /// 单次翻译请求超时（毫秒）。
    #[serde(rename = "request_timeout_ms", default = "default_request_timeout_ms")]
    request_timeout_ms: u64,
    /// 即时翻译模式下的防抖等待时间（秒），减少冗余请求。
    #[serde(rename = "live_debounce_secs", default = "default_live_debounce_secs")]
    live_debounce_secs: f64,
    /// 厂商预设；选非「自定义」时在 normalize 中写入对应 Base URL。
    #[serde(rename = "llm_vendor", default = "default_llm_vendor")]
    llm_vendor: String,
    /// 厂商预设列表（label → Base URL），用户可增删改；
    /// 选非「自定义」厂商时 normalize 依据本列表写入对应 Base URL。
    #[serde(rename = "llm_vendor_options", default = "default_llm_vendor_options")]
    llm_vendor_options: Vec<VendorPreset>,
    /// LLM 服务 Base URL。
    #[serde(rename = "llm_base_url", default)]
    llm_base_url: String,
    /// LLM API Key。
    #[serde(rename = "llm_api_key", default)]
    llm_api_key: String,
    /// LLM 模型名。
    #[serde(rename = "llm_model", default)]
    llm_model: String,
}

const TRANSLATE_MODE_LIVE: &str = "live";
const TRANSLATE_MODE_ON_ENTER: &str = "on_enter";

const LLM_VENDOR_CUSTOM: &str = "自定义";

/// 厂商预设（label → Base URL），随 llm_vendor_options 设置持久化，用户可增删改；
/// 「自定义」为隐式哨兵，不在此列表中。
#[derive(Debug, Clone, Serialize, Deserialize)]
struct VendorPreset {
    /// 厂商名称（持久化值，与 llm_vendor 字段一致）。
    #[serde(rename = "label")]
    label: String,
    /// 对应 Base URL；空串或 null 表示无预设地址
    /// （schema 的 string 类型不接受 null，统一以空串编码）。
    #[serde(rename = "url")]
    url: Option<String>,
}

/// 默认厂商预设（开箱即用的 OpenAI 兼容服务）。
/// 旧配置 JSON 无 llm_vendor_options 字段时经 serde default 自动播种。
fn default_llm_vendor_options() -> Vec<VendorPreset> {
    vec![
        VendorPreset {
            label: "DeepSeek".into(),
            url: Some("https://api.deepseek.com".into()),
        },
        VendorPreset {
            label: "智谱 GLM".into(),
            url: Some("https://open.bigmodel.cn/api/paas/v4".into()),
        },
        VendorPreset {
            label: "OpenAI".into(),
            url: Some("https://api.openai.com/v1".into()),
        },
        VendorPreset {
            label: "硅基流动".into(),
            url: Some("https://api.siliconflow.cn/v1".into()),
        },
        VendorPreset {
            label: "阿里云百炼".into(),
            url: Some("https://dashscope.aliyuncs.com/compatible-mode/v1".into()),
        },
        VendorPreset {
            label: "腾讯云 TokenHub".into(),
            url: Some("https://tokenhub.tencentmaas.com/v1".into()),
        },
        VendorPreset {
            label: "Kimi".into(),
            url: Some("https://api.moonshot.cn/v1".into()),
        },
        VendorPreset {
            label: "小米 MiMo".into(),
            url: Some("https://api.xiaomimimo.com/v1".into()),
        },
    ]
}

fn default_translate_mode() -> String {
    TRANSLATE_MODE_LIVE.into()
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

fn default_live_debounce_secs() -> f64 {
    0.5
}

fn default_llm_vendor() -> String {
    LLM_VENDOR_CUSTOM.into()
}

impl Default for TranslatorSettings {
    fn default() -> Self {
        Self {
            translate_mode: default_translate_mode(),
            default_target: default_target(),
            enabled_providers: default_enabled_providers(),
            request_timeout_ms: default_request_timeout_ms(),
            live_debounce_secs: default_live_debounce_secs(),
            llm_vendor: default_llm_vendor(),
            llm_vendor_options: default_llm_vendor_options(),
            llm_base_url: String::new(),
            llm_api_key: String::new(),
            llm_model: String::new(),
        }
    }
}

impl TranslatorSettings {
    /// 规范化：校验厂商预设列表与厂商选择，写入 Base URL。
    fn normalize(mut self) -> Self {
        if self.enabled_providers.is_empty() {
            self.enabled_providers = default_enabled_providers();
        }
        // 清理空 label 条目，避免产生无法选中的脏数据。
        self.llm_vendor_options
            .retain(|p| !p.label.trim().is_empty());
        // 厂商不在预设列表且非「自定义」（如对应预设被用户删除）→ 回落自定义。
        let vendor_known = self.llm_vendor == LLM_VENDOR_CUSTOM
            || self
                .llm_vendor_options
                .iter()
                .any(|p| p.label == self.llm_vendor);
        if !vendor_known {
            self.llm_vendor = default_llm_vendor();
        }
        // 选中预设厂商时写入对应 Base URL（用户预设列表为权威数据源）。
        // 空串（无预设地址）不写入，避免清空用户手填地址。
        if let Some(url) = self
            .llm_vendor_options
            .iter()
            .find(|p| p.label == self.llm_vendor)
            .and_then(|p| p.url.clone())
            .filter(|url| !url.is_empty())
        {
            self.llm_base_url = url;
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
        let llm_config = Arc::new(RwLock::new(LlmConfig::default()));
        let registry = ProviderRegistry::new(vec![
            Arc::new(OpenAiCompatibleProvider::new(Arc::clone(&llm_config))),
            Arc::new(MockProvider),
        ]);

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
            },
            inner: RwLock::new(TranslatorSettings::default()),
            llm_config,
            registry,
            last_result_text: RwLock::new(None),
            handle: RwLock::new(None),
        }
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

        let alternatives: Vec<serde_json::Value> =
            agg.alternatives.iter().map(Self::result_to_json).collect();

        let message = if has_primary {
            json!(null)
        } else {
            let detail = agg
                .primary
                .as_ref()
                .and_then(|p| p.error.clone())
                .or_else(|| agg.alternatives.iter().find_map(|a| a.error.clone()));
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
        let targets = self.active_language_support(&settings).targets;
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

        // 厂商预设 schema 选项：默认预设 label + 「自定义」。
        // translator 使用自定义设置面板（DynamicForm 不渲染此字段），此处仅作静态文档；
        // 运行时选项以持久化 llm_vendor_options 为准（用户可增删改）。
        let vendor_labels: Vec<String> = default_llm_vendor_options()
            .into_iter()
            .map(|p| p.label)
            .chain(std::iter::once(LLM_VENDOR_CUSTOM.to_string()))
            .collect();
        let vendor_refs: Vec<&str> = vendor_labels.iter().map(String::as_str).collect();

        // llm_vendor_options 字段声明：注册期 validate_settings 按键名校验 settings，
        // 未在 schema 声明的键会被拒绝（曾导致 translator 注册失败）。
        // 数组元素为 { label: string, url: string }；「无 URL」以空串编码（schema 无可空字符串类型）。
        let mut preset_item_properties = BTreeMap::new();
        preset_item_properties.insert(
            "label".to_string(),
            SchemaNode {
                kind: SchemaKind::String {
                    enum_values: Vec::new(),
                    enum_labels: Vec::new(),
                    min_length: Some(1),
                    max_length: None,
                    pattern: None,
                },
                default: None,
            },
        );
        preset_item_properties.insert(
            "url".to_string(),
            SchemaNode {
                kind: SchemaKind::String {
                    enum_values: Vec::new(),
                    enum_labels: Vec::new(),
                    min_length: None,
                    max_length: None,
                    pattern: None,
                },
                default: None,
            },
        );
        let llm_vendor_options_definition = SettingDefinition {
            key: "llm_vendor_options".to_string(),
            schema: SchemaNode {
                kind: SchemaKind::Array {
                    items: Box::new(SchemaNode {
                        kind: SchemaKind::Object {
                            properties: preset_item_properties,
                            ui: vec![
                                FieldUiMetadata {
                                    pointer: "/label".to_string(),
                                    label: "厂商名称".to_string(),
                                    description: "预设厂商名称（持久化值）".to_string(),
                                    group: None,
                                    order: 0,
                                    visible: true,
                                    read_only: false,
                                    widget: None,
                                    action: None,
                                    detail_action: None,
                                },
                                FieldUiMetadata {
                                    pointer: "/url".to_string(),
                                    label: "Base URL".to_string(),
                                    description: "预设厂商的 Base URL".to_string(),
                                    group: None,
                                    order: 1,
                                    visible: true,
                                    read_only: false,
                                    widget: None,
                                    action: None,
                                    detail_action: None,
                                },
                            ],
                            required: BTreeSet::from(["label".to_string()]),
                        },
                        default: None,
                    }),
                    item_widget: None,
                    min_items: None,
                    max_items: None,
                },
                default: None,
            },
            ui: FieldUiMetadata {
                pointer: "/llm_vendor_options".to_string(),
                label: "厂商预设列表".to_string(),
                description: "用户可增删改的厂商预设（label → Base URL）".to_string(),
                group: Some("LLM 服务".to_string()),
                order: 13,
                visible: true,
                read_only: false,
                widget: None,
                action: None,
                detail_action: None,
            },
        };

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
            SchemaBuilder::array(
                "enabled_providers",
                t_key!("translator", "fields.enabled_providers.label"),
                t_key!("translator", "fields.enabled_providers.desc"),
            )
            .primitive_item(PrimitiveType::Select {
                options: vec![PROVIDER_ID.into(), MOCK_PROVIDER_ID.into()],
            })
            .group(t_key!("translator", "groups.engine"))
            .order(2)
            .default(json!([PROVIDER_ID]))
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
            .order(3)
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
            SchemaBuilder::select(
                "llm_vendor",
                t_key!("translator", "fields.llm_vendor.label"),
                t_key!("translator", "fields.llm_vendor.desc"),
            )
            .options(&vendor_refs)
            .group(t_key!("translator", "groups.llmService"))
            .order(9)
            .default(LLM_VENDOR_CUSTOM)
            .build(),
            SchemaBuilder::text(
                "llm_base_url",
                t_key!("translator", "fields.llm_base_url.label"),
                t_key!("translator", "fields.llm_base_url.desc"),
            )
            .group(t_key!("translator", "groups.llmService"))
            .order(10)
            .default("")
            .build(),
            SchemaBuilder::text(
                "llm_api_key",
                t_key!("translator", "fields.llm_api_key.label"),
                t_key!("translator", "fields.llm_api_key.desc"),
            )
            .group(t_key!("translator", "groups.llmService"))
            .order(11)
            .default("")
            .build(),
            SchemaBuilder::text(
                "llm_model",
                t_key!("translator", "fields.llm_model.label"),
                t_key!("translator", "fields.llm_model.desc"),
            )
            .group(t_key!("translator", "groups.llmService"))
            .order(12)
            .default("")
            .build(),
            llm_vendor_options_definition,
        ]
    }

    fn get_settings(&self) -> serde_json::Value {
        serde_json::to_value(self.inner.read().clone()).unwrap_or_default()
    }

    async fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        let parsed = serde_json::from_value::<TranslatorSettings>(settings)
            .unwrap_or_default()
            .normalize();
        self.sync_llm_config(&parsed);
        *self.inner.write() = parsed;
        Ok(())
    }

    fn get_default_settings(&self) -> serde_json::Value {
        serde_json::to_value(TranslatorSettings::default()).unwrap_or_default()
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
        // 保存服务句柄，供 execute_action 经 PluginHandle 访问平台能力（如剪贴板）。
        *self.handle.write() = handle;
        let settings = self.inner.read().clone();
        self.sync_llm_config(&settings);
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

        let agg = self
            .registry
            .translate_all(
                &req,
                &settings.enabled_providers,
                settings.preferred_provider_id(),
                settings.request_timeout_ms,
                &ctx.trace_id,
                ctx.query_revision(),
            )
            .await;

        // 缓存译文文本，供 execute_action 写入剪贴板。
        // 仅 GUI 通道且查询仍最新时写入：CLI/调试查询为只读辅助路径，
        // 不得改写 GUI 剪贴板缓存（execute_action 无通道区分，
        // 复制动作必须始终拿到与 GUI 面板一致的译文）。
        if ctx.is_query_current() && ctx.query_channel == QueryChannel::Ui {
            *self.last_result_text.write() = agg
                .primary
                .as_ref()
                .filter(|r| r.is_success())
                .map(|r| r.text.clone());
        } else {
            info!(
                trace_id = %ctx.trace_id,
                query_revision = ctx.query_revision(),
                site = "plugin_cache",
                "查询过期，丢弃翻译结果缓存写入"
            );
        }

        Ok(Self::aggregate_to_panel(&parsed, agg))
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
                "enabled_providers": [PROVIDER_ID],
                "request_timeout_ms": 15000,
                "llm_vendor": LLM_VENDOR_CUSTOM,
                "llm_base_url": "",
                "llm_api_key": "",
                "llm_model": "",
            }))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn query_without_credentials_returns_error_panel() {
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

        // 确认查询（Enter 触发，confirm=true）→ 翻译路径（无凭据 → error）
        let second = plugin
            .query(&ctx, &sample_query_with_confirm("hello", true))
            .await
            .unwrap();
        match second {
            QueryResponse::CustomPanel { data, .. } => {
                assert_eq!(data["status"], "error");
                let msg = data["message"].as_str().unwrap_or("");
                assert!(
                    msg.contains("设置") || msg.contains("填写"),
                    "期望进入 LLM 路径的配置错误，实际: {msg}"
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

    #[tokio::test]
    /// 过期查询（更新的查询已进入后端）不得覆盖共享译文缓存。
    async fn stale_query_does_not_overwrite_result_cache() {
        use std::sync::atomic::AtomicU64;
        use std::sync::Arc;
        use zerolaunch_plugin_api::QueryRevisionGate;

        let plugin = TranslatorPlugin::new();
        plugin
            .apply_settings(json!({
                "translate_mode": TRANSLATE_MODE_LIVE,
                "default_target": "zh",
                "enabled_providers": [MOCK_PROVIDER_ID],
            }))
            .await
            .unwrap();

        let latest = Arc::new(AtomicU64::new(2));

        // 当前查询（revision=2，与最新一致）：正常写入译文缓存。
        let mut ctx = PluginContext::new("test");
        ctx.set_query_revision_gate(QueryRevisionGate::new(2, latest.clone()));
        plugin.query(&ctx, &sample_query("hello")).await.unwrap();
        assert!(
            plugin.last_result_text.read().is_some(),
            "最新查询应写入译文缓存"
        );

        // 过期查询（revision=1，最新已推进到 2）：翻译照常执行，但不得覆盖缓存。
        let mut stale_ctx = PluginContext::new("test");
        stale_ctx.set_query_revision_gate(QueryRevisionGate::new(1, latest));
        let resp = plugin
            .query(&stale_ctx, &sample_query("world"))
            .await
            .unwrap();
        match resp {
            QueryResponse::CustomPanel { data, .. } => {
                assert_eq!(data["status"], "ok");
            }
            other => panic!("期望 CustomPanel，实际 {:?}", other),
        }
        assert_eq!(
            plugin.last_result_text.read().as_deref(),
            Some("模拟示例占位译文"),
            "过期查询不得覆盖最新译文缓存"
        );

        // 无门控上下文（远端插件/测试默认）：恒为最新，正常写入。
        let plain_ctx = PluginContext::new("test");
        plugin
            .query(&plain_ctx, &sample_query("again"))
            .await
            .unwrap();
        assert!(
            plugin.last_result_text.read().is_some(),
            "无门控上下文应视为最新并写入缓存"
        );
    }

    #[tokio::test]
    /// CLI 通道查询（只读辅助路径）不得写入 GUI 剪贴板缓存：
    /// execute_action 复制动作无通道区分，缓存必须始终对应 GUI 面板译文。
    async fn cli_channel_query_does_not_write_result_cache() {
        use zerolaunch_plugin_api::QueryChannel;

        let plugin = TranslatorPlugin::new();
        plugin
            .apply_settings(json!({
                "translate_mode": TRANSLATE_MODE_LIVE,
                "default_target": "zh",
                "enabled_providers": [MOCK_PROVIDER_ID],
            }))
            .await
            .unwrap();

        // CLI 查询：翻译照常执行，但不得创建/改写剪贴板缓存。
        let cli_ctx = PluginContext {
            query_channel: QueryChannel::Cli,
            ..PluginContext::new("test")
        };
        let resp = plugin
            .query(&cli_ctx, &sample_query("hello"))
            .await
            .unwrap();
        match resp {
            QueryResponse::CustomPanel { data, .. } => {
                assert_eq!(data["status"], "ok");
            }
            other => panic!("期望 CustomPanel，实际 {:?}", other),
        }
        assert!(
            plugin.last_result_text.read().is_none(),
            "CLI 通道查询不得写入剪贴板缓存"
        );

        // GUI 查询：正常写入缓存（复制动作的数据来源）。
        let ui_ctx = PluginContext::new("test");
        plugin.query(&ui_ctx, &sample_query("hello")).await.unwrap();
        assert!(
            plugin.last_result_text.read().is_some(),
            "GUI 通道查询应写入剪贴板缓存"
        );
    }

    #[tokio::test]
    async fn kimi_vendor_fills_moonshot_base_url() {
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

    #[test]
    /// 用户新增预设：选中后 normalize 写入对应 Base URL（预设列表为权威数据源）。
    fn user_defined_preset_writes_base_url() {
        let settings = TranslatorSettings {
            llm_vendor: "MyVendor".into(),
            llm_base_url: String::new(),
            llm_vendor_options: vec![VendorPreset {
                label: "MyVendor".into(),
                url: Some("https://my.example.com/v1".into()),
            }],
            ..TranslatorSettings::default()
        }
        .normalize();
        assert_eq!(settings.llm_base_url, "https://my.example.com/v1");
        assert_eq!(settings.llm_vendor, "MyVendor");
    }

    #[test]
    /// 预设被用户删除后，已选厂商回落「自定义」，不写入地址也不清空手填 URL。
    fn deleted_preset_falls_back_to_custom() {
        let settings = TranslatorSettings {
            llm_vendor: "Kimi".into(),
            llm_base_url: "https://example.com/v1".into(),
            llm_vendor_options: vec![],
            ..TranslatorSettings::default()
        }
        .normalize();
        assert_eq!(settings.llm_vendor, LLM_VENDOR_CUSTOM);
        assert_eq!(settings.llm_base_url, "https://example.com/v1");
    }

    #[test]
    /// 旧配置 JSON 无 llm_vendor_options 字段：serde default 自动播种默认预设。
    fn missing_vendor_options_field_seeds_defaults() {
        let settings: TranslatorSettings =
            serde_json::from_value(json!({ "llm_vendor": "Kimi" })).unwrap();
        assert_eq!(settings.llm_vendor_options.len(), 8);
        assert_eq!(settings.llm_vendor_options[0].label, "DeepSeek");
        assert_eq!(
            settings.llm_vendor_options[0].url.as_deref(),
            Some("https://api.deepseek.com")
        );
    }

    #[tokio::test]
    /// 用户预设随设置持久化往返：apply_settings 保存后 get_settings 原样下发。
    async fn vendor_options_persist_through_apply_and_get() {
        let plugin = TranslatorPlugin::new();
        plugin
            .apply_settings(json!({
                "llm_vendor": "MyVendor",
                "llm_vendor_options": [
                    { "label": "MyVendor", "url": "https://my.example.com/v1" },
                ],
            }))
            .await
            .unwrap();
        let value = plugin.get_settings();
        assert_eq!(value["llm_vendor"], "MyVendor");
        assert_eq!(value["llm_vendor_options"][0]["label"], "MyVendor");
        assert_eq!(
            value["llm_vendor_options"][0]["url"],
            "https://my.example.com/v1"
        );
        // normalize 依据用户预设写入 Base URL
        assert_eq!(value["llm_base_url"], "https://my.example.com/v1");
    }

    #[test]
    /// 空 label 预设条目在 normalize 中被清理，避免产生无法选中的脏数据。
    fn empty_label_presets_are_cleaned() {
        let settings = TranslatorSettings {
            llm_vendor_options: vec![
                VendorPreset {
                    label: "  ".into(),
                    url: Some("https://x.example.com".into()),
                },
                VendorPreset {
                    label: "OK".into(),
                    url: None,
                },
            ],
            ..TranslatorSettings::default()
        }
        .normalize();
        assert_eq!(settings.llm_vendor_options.len(), 1);
        assert_eq!(settings.llm_vendor_options[0].label, "OK");
    }
}
