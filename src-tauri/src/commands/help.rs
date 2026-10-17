//! AI 助手 IPC：设置检索（本地确定性匹配）与问答（模型 + 只读设置工具）。
//!
//! `help_search` 不调用模型，命中结果可直接跳转到对应设置项；
//! `help_ask` 走 `core::agent` 的最小工具循环，模型由 `help-agent-config` 指定。
//! 命令壳只负责生成 trace_id 与错误映射，实体逻辑在 `search_hits` / `run_ask`。

use crate::commands::bridge_error::{BridgeError, WithTraceId};
use crate::core::agent::{ask_from_config, AgentError, SettingsManual};
use crate::core::config::ConfigManager;
use crate::core::i18n::I18nManager;
use crate::state::app_state::AppState;
use crate::utils::trace_id::generate_trace_id;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use zerolaunch_plugin_api::services::model::ModelService;

/// 检索返回的命中条数上限。
const SEARCH_LIMIT: usize = 20;

/// IPC `help_search` 返回的命中条目。
#[derive(Debug, Serialize)]
pub struct HelpHitDto {
    #[serde(rename = "componentId")]
    pub component_id: String,
    #[serde(rename = "fieldKey")]
    pub field_key: String,
    #[serde(rename = "label")]
    pub label: String,
}

/// IPC `help_ask` 返回的答复。
#[derive(Debug, Serialize)]
pub struct HelpAnswerDto {
    #[serde(rename = "answer")]
    pub answer: String,
    #[serde(rename = "references")]
    pub references: Vec<HelpReferenceDto>,
}

/// 答复中引用到的设置位置（前端渲染为可跳转链接）。
#[derive(Debug, Serialize)]
pub struct HelpReferenceDto {
    #[serde(rename = "componentId")]
    pub component_id: String,
    #[serde(rename = "componentName")]
    pub component_name: String,
    #[serde(rename = "fieldKey")]
    pub field_key: String,
    #[serde(rename = "fieldLabel")]
    pub field_label: String,
}

/// `help_ask` 入参：组件的设置页点击路径（前端按当前界面语言拼好）。
#[derive(Debug, serde::Deserialize)]
pub struct HelpNavigationDto {
    #[serde(rename = "componentId")]
    pub component_id: String,
    #[serde(rename = "path")]
    pub path: String,
}

/// 检索设置字段。
///
/// 参数：state - 应用状态；query - 关键词。
/// 返回：按相关度降序排列的命中条目；空查询返回空列表。
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub fn help_search(
    state: tauri::State<'_, Arc<AppState>>,
    query: String,
) -> Result<Vec<HelpHitDto>, BridgeError> {
    let trace_id = generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());
    let hits = search_hits(
        &state.get_config_manager(),
        &state.get_i18n_manager(),
        &query,
    );
    Ok::<_, BridgeError>(hits).with_trace_id(&trace_id)
}

/// 向 AI 助手提问。
///
/// 参数：state - 应用状态；question - 用户问题；
/// navigation - 组件的设置页点击路径（可选；用于让答复给出逐步导航）。
/// 返回：答复文本与有效引用；未配置模型、模型不在清单或模型调用失败时返回错误。
#[tauri::command]
#[tracing::instrument(skip(state), fields(trace_id))]
pub async fn help_ask(
    state: tauri::State<'_, Arc<AppState>>,
    question: String,
    navigation: Option<Vec<HelpNavigationDto>>,
) -> Result<HelpAnswerDto, BridgeError> {
    let trace_id = generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());
    let navigation: HashMap<String, String> = navigation
        .unwrap_or_default()
        .into_iter()
        .map(|entry| (entry.component_id, entry.path))
        .collect();
    let answer = run_ask(
        state.get_model_manager(),
        state.get_config_manager(),
        state.get_i18n_manager(),
        &question,
        &navigation,
    )
    .await
    .with_trace_id(&trace_id)?;
    Ok(answer)
}

/// 检索实体：构建说明书（schema + 译文）后按关键词匹配。
///
/// 参数：config - 配置管理器；i18n - 翻译服务；query - 关键词。
/// 返回：命中条目；空查询返回空列表。
fn search_hits(config: &ConfigManager, i18n: &I18nManager, query: &str) -> Vec<HelpHitDto> {
    SettingsManual::build(config, i18n)
        .search(query, SEARCH_LIMIT)
        .into_iter()
        .map(|hit| HelpHitDto {
            component_id: hit.component_id,
            field_key: hit.field_key,
            label: hit.label,
        })
        .collect()
}

/// 问答实体：驱动 `core::agent` 的配置解析与工具循环，并把结果映射为 IPC DTO。
///
/// 参数：service - 模型服务；config - 配置管理器；i18n - 翻译服务；
/// question - 用户问题；navigation - 组件 id → 设置页点击路径。
/// 返回：答复与引用；失败时返回助手错误（由命令层经 `From` 转 `BridgeError`）。
async fn run_ask(
    service: Arc<dyn ModelService>,
    config: Arc<ConfigManager>,
    i18n: Arc<I18nManager>,
    question: &str,
    navigation: &HashMap<String, String>,
) -> Result<HelpAnswerDto, AgentError> {
    let answer = ask_from_config(service, config, i18n, question, navigation).await?;
    let references = answer
        .references
        .into_iter()
        .map(|reference| HelpReferenceDto {
            component_id: reference.component_id,
            component_name: reference.component_name,
            field_key: reference.field_key,
            field_label: reference.field_label,
        })
        .collect();
    Ok(HelpAnswerDto {
        answer: answer.answer,
        references,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builtin_plugin::config::help_agent_config::HelpAgentConfigComponent;
    use crate::commands::bridge_error::ErrorCode;
    use crate::core::agent::{HELP_AGENT_CONFIG_ID, MASK};
    use crate::core::config::setting_builders::SchemaBuilder;
    use async_trait::async_trait;
    use parking_lot::{Mutex, RwLock};
    use std::path::Path;
    use zerolaunch_plugin_api::config::{
        ComponentCore, ComponentType, ConfigError, Configurable, SettingDefinition,
    };
    use zerolaunch_plugin_api::mock::helpers::mock_plugin_handle;
    use zerolaunch_plugin_api::services::model::{
        ChatCapability, ModelChatRequest, ModelChatResponse, ModelChatRole, ModelEmbeddingRequest,
        ModelEmbeddingResponse, ModelError, ModelInfo, ModelKind, ModelSimilarityRequest,
        ModelSimilarityResponse, ModelStreamChunk, ModelToolCall,
    };

    /// 测试用配置组件：一个带分组的普通数值字段 + 一个敏感字段。
    struct DemoConfigComponent {
        core: ComponentCore,
        settings: RwLock<DemoSettings>,
    }

    /// 演示组件的强类型设置。
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    struct DemoSettings {
        #[serde(rename = "threshold", default = "default_threshold")]
        threshold: f64,
        #[serde(rename = "api_key", default = "default_api_key")]
        api_key: String,
    }

    /// 演示阈值默认值（同时作为「当前值」参与断言）。
    fn default_threshold() -> f64 {
        0.75
    }

    /// 演示密钥默认值（用于断言脱敏后不出现在上下文里）。
    fn default_api_key() -> String {
        "sk-secret".to_string()
    }

    impl Default for DemoSettings {
        fn default() -> Self {
            Self {
                threshold: default_threshold(),
                api_key: default_api_key(),
            }
        }
    }

    impl DemoConfigComponent {
        fn new() -> Self {
            Self {
                // 文案用字面量：说明书按 key-or-literal 渲染，字面量直接出现在手册里
                core: ComponentCore::new(
                    "demo-config".to_string(),
                    "演示设置".to_string(),
                    "测试用组件".to_string(),
                    ComponentType::Core,
                    99,
                ),
                settings: RwLock::new(DemoSettings::default()),
            }
        }
    }

    #[async_trait]
    impl Configurable for DemoConfigComponent {
        fn core(&self) -> &ComponentCore {
            &self.core
        }

        fn setting_schema(&self) -> Vec<SettingDefinition> {
            vec![
                SchemaBuilder::number("threshold", "阈值", "匹配阈值")
                    .group("演示分组")
                    .default(default_threshold())
                    .build(),
                SchemaBuilder::text("api_key", "密钥", "接口密钥")
                    .sensitive(true)
                    .default("")
                    .build(),
            ]
        }

        fn get_settings(&self) -> serde_json::Value {
            serde_json::to_value(self.settings.read().clone()).unwrap_or_default()
        }

        async fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
            let parsed: DemoSettings = serde_json::from_value(settings).unwrap_or_default();
            *self.settings.write() = parsed;
            Ok(())
        }
    }

    /// 脚本化模型服务：按序返回预设响应，并记录收到的请求。
    struct ScriptedModelService {
        model_id: String,
        responses: Mutex<Vec<ModelChatResponse>>,
        requests: Mutex<Vec<ModelChatRequest>>,
    }

    impl ScriptedModelService {
        fn new(model_id: &str, responses: Vec<ModelChatResponse>) -> Self {
            Self {
                model_id: model_id.to_string(),
                responses: Mutex::new(responses),
                requests: Mutex::new(Vec::new()),
            }
        }

        /// 已收到的请求（按序）。
        fn requests(&self) -> Vec<ModelChatRequest> {
            self.requests.lock().clone()
        }
    }

    #[async_trait]
    impl ModelService for ScriptedModelService {
        fn list_models(&self) -> Vec<ModelInfo> {
            vec![chat_model_info(&self.model_id)]
        }

        fn model_info(&self, model_id: &str) -> Option<ModelInfo> {
            (model_id == self.model_id).then(|| chat_model_info(&self.model_id))
        }

        async fn chat(&self, req: ModelChatRequest) -> Result<ModelChatResponse, ModelError> {
            self.requests.lock().push(req.clone());
            let mut responses = self.responses.lock();
            if responses.is_empty() {
                return Ok(ModelChatResponse {
                    model_id: req.model_id,
                    content: "（脚本响应已用尽）".to_string(),
                    finish_reason: None,
                    usage: None,
                    tool_calls: Vec::new(),
                });
            }
            Ok(responses.remove(0))
        }

        async fn stream_chat(
            &self,
            _req: ModelChatRequest,
            _tx: tokio::sync::mpsc::Sender<ModelStreamChunk>,
        ) -> Result<(), ModelError> {
            Err(ModelError::NotSupported)
        }

        async fn embedding(
            &self,
            _req: ModelEmbeddingRequest,
        ) -> Result<ModelEmbeddingResponse, ModelError> {
            Err(ModelError::NotSupported)
        }

        async fn similarity(
            &self,
            _req: ModelSimilarityRequest,
        ) -> Result<ModelSimilarityResponse, ModelError> {
            Err(ModelError::NotSupported)
        }
    }

    /// 构造声明了 tools 能力的 chat 模型元数据。
    fn chat_model_info(model_id: &str) -> ModelInfo {
        ModelInfo {
            model_id: model_id.to_string(),
            name: model_id.to_string(),
            kind: ModelKind::Chat {
                context_window: None,
                max_output: None,
                supports_stream: false,
                capabilities: vec![ChatCapability::Tools],
            },
            provider: model_id
                .split_once('/')
                .map(|(provider, _)| provider.to_string())
                .unwrap_or_default(),
        }
    }

    /// 写入临时语言包并加载翻译服务（固定为简体中文）。
    fn test_i18n(dir: &Path, catalog: serde_json::Value) -> Arc<I18nManager> {
        std::fs::write(dir.join("zh-Hans.json"), catalog.to_string()).unwrap();
        let manager = I18nManager::load(dir.to_path_buf());
        manager.set_language("zh-Hans");
        manager
    }

    /// 注册演示组件与真实助手配置组件；model_id 非空时写入助手配置。
    async fn test_config(dir: &Path, model_id: &str) -> Arc<ConfigManager> {
        let config = Arc::new(ConfigManager::new(dir.to_path_buf()));
        config.register(Arc::new(DemoConfigComponent::new())).await;
        config
            .register(Arc::new(
                HelpAgentConfigComponent::new(mock_plugin_handle()),
            ))
            .await;
        // 显式写入演示组件的「当前值」：证明真实取值进入上下文、密钥被遮蔽
        config
            .apply_settings(
                "demo-config",
                serde_json::json!({
                    "threshold": default_threshold(),
                    "api_key": default_api_key(),
                }),
            )
            .await
            .unwrap();
        if !model_id.is_empty() {
            config
                .apply_settings(
                    HELP_AGENT_CONFIG_ID,
                    serde_json::json!({ "model_id": model_id }),
                )
                .await
                .unwrap();
        }
        config
    }

    /// 设置页点击路径（前端提供；测试用固定中文路径）。
    fn navigation() -> HashMap<String, String> {
        HashMap::from([(
            "demo-config".to_string(),
            "设置 › 常规设置 › 演示设置".to_string(),
        )])
    }

    /// 前端入参契约：navigation 用 camelCase 键名。
    #[test]
    fn navigation_dto_reads_frontend_payload() {
        let dto: HelpNavigationDto = serde_json::from_value(serde_json::json!({
            "componentId": "appearance-config",
            "path": "设置 › 外观设置 › 外观",
        }))
        .unwrap();
        assert_eq!(dto.component_id, "appearance-config");
        assert_eq!(dto.path, "设置 › 外观设置 › 外观");
    }

    /// 助手配置组件的语言包（说明书按 i18n key 解析译文）。
    fn help_agent_catalog() -> serde_json::Value {
        serde_json::json!({
            "components": {
                "help-agent-config": {
                    "name": "AI 助手",
                    "description": "用自然语言询问设置用法",
                    "groups": { "assistant": "助手" },
                    "fields": {
                        "model_id": { "label": "助手模型", "desc": "助手使用的模型 id" }
                    }
                }
            }
        })
    }

    #[tokio::test]
    async fn search_hits_matches_label_and_key_and_skips_blank_query() {
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path(), "ollama/test-model").await;
        let i18n = test_i18n(dir.path(), help_agent_catalog());

        let by_label = search_hits(&config, &i18n, "阈值");
        assert_eq!(by_label.len(), 1);
        assert_eq!(by_label[0].component_id, "demo-config");
        assert_eq!(by_label[0].field_key, "threshold");
        assert_eq!(by_label[0].label, "阈值");

        let by_key = search_hits(&config, &i18n, "api_key");
        assert_eq!(by_key.len(), 1);
        assert_eq!(by_key[0].field_key, "api_key");

        assert!(search_hits(&config, &i18n, "   ").is_empty());
        assert!(search_hits(&config, &i18n, "不存在的关键词").is_empty());
    }

    /// 带工具调用的一轮里写的正文必须保留：模型可能在这一轮先给出前半段（含点击路径），
    /// 下一轮才补剩下的内容；丢掉首轮正文会让答复缺前半段。
    #[tokio::test]
    async fn ask_keeps_prose_written_alongside_tool_calls() {
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path(), "ollama/test-model").await;
        let i18n = test_i18n(dir.path(), help_agent_catalog());
        let service = Arc::new(ScriptedModelService::new(
            "ollama/test-model",
            vec![
                ModelChatResponse {
                    model_id: "ollama/test-model".to_string(),
                    content: "**1. 支持情况与设置位置**\n- 支持，路径 `设置 → 常规设置 → 演示设置 → 阈值`。"
                        .to_string(),
                    finish_reason: Some("tool_calls".to_string()),
                    usage: None,
                    tool_calls: vec![ModelToolCall {
                        id: "call-1".to_string(),
                        name: "get_settings".to_string(),
                        arguments: r#"{"component_id":"demo-config"}"#.to_string(),
                    }],
                },
                ModelChatResponse {
                    model_id: "ollama/test-model".to_string(),
                    content: "**2. 当前值与改后效果**\n- `threshold = 0.75`。\n相关设置: demo-config/threshold"
                        .to_string(),
                    finish_reason: Some("stop".to_string()),
                    usage: None,
                    tool_calls: Vec::new(),
                },
            ],
        ));

        let answer = run_ask(service, config, i18n, "阈值是多少？", &navigation())
            .await
            .with_trace_id("test-trace")
            .unwrap();
        assert!(
            answer.answer.contains("**1. 支持情况与设置位置**"),
            "首轮正文被丢弃：{}",
            answer.answer
        );
        assert!(
            answer.answer.contains("演示设置 → 阈值"),
            "{}",
            answer.answer
        );
        assert!(
            answer.answer.contains("**2. 当前值与改后效果**"),
            "{}",
            answer.answer
        );
        // 引用行仍被剥离，只保留真实存在的字段引用
        assert!(!answer.answer.contains("相关设置"), "{}", answer.answer);
        assert_eq!(answer.references.len(), 1);
        assert_eq!(answer.references[0].field_key, "threshold");
    }

    #[tokio::test]
    async fn ask_drives_tool_round_trip_masks_secret_and_maps_references() {
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path(), "ollama/test-model").await;
        let i18n = test_i18n(dir.path(), help_agent_catalog());
        let service = Arc::new(ScriptedModelService::new(
            "ollama/test-model",
            vec![
                ModelChatResponse {
                    model_id: "ollama/test-model".to_string(),
                    content: String::new(),
                    finish_reason: Some("tool_calls".to_string()),
                    usage: None,
                    tool_calls: vec![ModelToolCall {
                        id: "call-1".to_string(),
                        name: "get_settings".to_string(),
                        arguments: r#"{"component_id":"demo-config"}"#.to_string(),
                    }],
                },
                ModelChatResponse {
                    model_id: "ollama/test-model".to_string(),
                    content: "阈值当前是 0.75。\n相关设置: demo-config/threshold, demo-config/api_key, nowhere/x"
                        .to_string(),
                    finish_reason: Some("stop".to_string()),
                    usage: None,
                    tool_calls: Vec::new(),
                },
            ],
        ));

        let answer = run_ask(
            service.clone(),
            config.clone(),
            i18n,
            "阈值是多少？",
            &navigation(),
        )
        .await
        .with_trace_id("test-trace")
        .unwrap();

        // 引用行从答复中剔除，臆造位置被丢弃，标签取说明书译文
        assert_eq!(answer.answer, "阈值当前是 0.75。");
        assert_eq!(answer.references.len(), 2);
        assert_eq!(answer.references[0].component_id, "demo-config");
        assert_eq!(answer.references[0].component_name, "演示设置");
        assert_eq!(answer.references[0].field_key, "threshold");
        assert_eq!(answer.references[0].field_label, "阈值");
        assert_eq!(answer.references[1].field_key, "api_key");
        assert_eq!(answer.references[1].field_label, "密钥");

        let requests = service.requests();
        assert_eq!(requests.len(), 2, "应先调用工具，再基于结果给出答复");

        // 第一轮：系统提示词含说明书（i18n key 已解析）+ 下发的工具清单
        let system = &requests[0].messages[0];
        assert_eq!(system.role, ModelChatRole::System);
        assert!(system.content.contains("AI 助手"), "说明书应解析 i18n key");
        assert!(system.content.contains("阈值"), "说明书应含字段标签");
        assert!(system.content.contains("敏感字段"), "说明书应标注敏感字段");
        assert!(
            system
                .content
                .contains("设置页路径：设置 › 常规设置 › 演示设置"),
            "说明书应含前端下发的设置页路径"
        );
        assert!(
            system.content.contains("`threshold` 阈值｜位置：演示分组"),
            "说明书应含字段所在分组（模型据此补出组件内的中间一步）"
        );
        assert_eq!(requests[0].tools.len(), 1);
        assert_eq!(requests[0].tools[0].name, "get_settings");
        assert_eq!(
            requests[0].tools[0].parameters["required"][0],
            "component_id"
        );

        // 第二轮：工具结果按调用 id 配对回填，敏感值脱敏
        let tool_message = requests[1]
            .messages
            .iter()
            .find(|message| message.role == ModelChatRole::Tool)
            .expect("应有工具结果消息");
        assert_eq!(tool_message.tool_call_id.as_deref(), Some("call-1"));
        assert!(
            tool_message.content.contains(MASK),
            "敏感字段必须脱敏: {}",
            tool_message.content
        );
        assert!(
            !tool_message.content.contains("sk-secret"),
            "明文密钥不得进入上下文"
        );
        assert!(
            tool_message.content.contains(r#""threshold":0.75"#),
            "工具结果内容: {}",
            tool_message.content
        );
    }

    #[tokio::test]
    async fn ask_rejects_blank_question_without_calling_model() {
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path(), "ollama/test-model").await;
        let i18n = test_i18n(dir.path(), help_agent_catalog());
        let service = Arc::new(ScriptedModelService::new("ollama/test-model", Vec::new()));

        let error = run_ask(service.clone(), config, i18n, "   ", &navigation())
            .await
            .with_trace_id("test-trace")
            .unwrap_err();
        assert!(matches!(error.code, ErrorCode::ValidationFailed));
        assert!(service.requests().is_empty(), "空问题不应触发模型调用");
    }

    #[tokio::test]
    async fn ask_rejects_unconfigured_and_unknown_model() {
        let dir = tempfile::tempdir().unwrap();
        let i18n = test_i18n(dir.path(), help_agent_catalog());
        let service = Arc::new(ScriptedModelService::new("ollama/test-model", Vec::new()));

        // 未配置模型：助手配置为空
        let unconfigured = test_config(dir.path(), "").await;
        let error = run_ask(
            service.clone(),
            unconfigured,
            i18n.clone(),
            "阈值是多少？",
            &navigation(),
        )
        .await
        .with_trace_id("test-trace")
        .unwrap_err();
        assert!(matches!(error.code, ErrorCode::ValidationFailed));
        assert!(error.message.contains("未配置模型"), "{}", error.message);

        // 模型不在清单：配置指向清单外的 id
        let unknown = test_config(dir.path(), "ollama/missing-model").await;
        let error = run_ask(
            service.clone(),
            unknown,
            i18n,
            "阈值是多少？",
            &navigation(),
        )
        .await
        .with_trace_id("test-trace")
        .unwrap_err();
        assert!(matches!(error.code, ErrorCode::ValidationFailed));
        assert!(error.message.contains("missing-model"), "{}", error.message);
        assert!(service.requests().is_empty(), "模型校验失败不应触发调用");
    }

    #[tokio::test]
    async fn ask_maps_provider_failure_to_network_error() {
        let dir = tempfile::tempdir().unwrap();
        let config = test_config(dir.path(), "ollama/test-model").await;
        let i18n = test_i18n(dir.path(), help_agent_catalog());

        /// 一律返回提供方不可用的模型服务。
        struct UnavailableService;

        #[async_trait]
        impl ModelService for UnavailableService {
            fn list_models(&self) -> Vec<ModelInfo> {
                vec![chat_model_info("ollama/test-model")]
            }
            fn model_info(&self, model_id: &str) -> Option<ModelInfo> {
                (model_id == "ollama/test-model").then(|| chat_model_info("ollama/test-model"))
            }
            async fn chat(&self, _req: ModelChatRequest) -> Result<ModelChatResponse, ModelError> {
                Err(ModelError::ProviderUnavailable("ollama 未启动".to_string()))
            }
            async fn stream_chat(
                &self,
                _req: ModelChatRequest,
                _tx: tokio::sync::mpsc::Sender<ModelStreamChunk>,
            ) -> Result<(), ModelError> {
                Err(ModelError::NotSupported)
            }
            async fn embedding(
                &self,
                _req: ModelEmbeddingRequest,
            ) -> Result<ModelEmbeddingResponse, ModelError> {
                Err(ModelError::NotSupported)
            }
            async fn similarity(
                &self,
                _req: ModelSimilarityRequest,
            ) -> Result<ModelSimilarityResponse, ModelError> {
                Err(ModelError::NotSupported)
            }
        }

        let error = run_ask(
            Arc::new(UnavailableService),
            config,
            i18n,
            "阈值是多少？",
            &navigation(),
        )
        .await
        .with_trace_id("test-trace")
        .unwrap_err();
        assert!(matches!(error.code, ErrorCode::NetworkError));
        assert_eq!(error.trace_id, "test-trace");
    }
}
