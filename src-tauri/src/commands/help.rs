//! AI 助手 IPC：设置检索（本地确定性匹配）与问答（模型 + 只读设置工具）。
//!
//! `help_search` 不调用模型，命中结果可直接跳转到对应设置项；
//! `help_ask` 走 `core::agent` 的最小工具循环，模型由 `help-agent-config` 指定。
//! 命令壳只负责生成 trace_id 与错误映射，实体逻辑在 `search_hits` / `run_ask`。

use crate::commands::bridge_error::{BridgeError, WithTraceId};
use crate::core::agent::{ask_from_config, AgentError, AskRequest, SettingsManual};
use crate::core::config::ConfigManager;
use crate::core::i18n::I18nManager;
use crate::state::app_state::AppState;
use crate::utils::trace_id::generate_trace_id;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use zerolaunch_plugin_api::services::model::{ModelChatMessage, ModelChatRole};

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

/// `help_ask` 入参：一条历史消息（本次提问之前的用户提问或助手答复）。
///
/// 只接受 `user` / `assistant` 两种角色，其余角色在命令层丢弃，
/// 避免前端越过本入口构造 system / tool 消息。
#[derive(Debug, serde::Deserialize)]
pub struct HelpMessageDto {
    /// 角色，取值 `user` 或 `assistant`。
    #[serde(rename = "role")]
    pub role: String,
    /// 消息正文。
    #[serde(rename = "content")]
    pub content: String,
}

/// `help_ask` 入参（三个字段合并为单个结构体，前端按 `payload` 键下发）。
#[derive(Debug, serde::Deserialize)]
pub struct HelpAskPayload {
    /// 用户问题。
    #[serde(rename = "question")]
    pub question: String,
    /// 组件的设置页点击路径（可选；用于让答复给出逐步导航）。
    #[serde(rename = "navigation")]
    pub navigation: Option<Vec<HelpNavigationDto>>,
    /// 本次提问之前的对话（可选；多轮澄清后据此整理 issue）。
    #[serde(rename = "history")]
    pub history: Option<Vec<HelpMessageDto>>,
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
/// 参数：state - 应用状态；payload - 提问入参（问题、设置页路径、历史对话）。
/// 返回：答复文本与有效引用；未配置模型、模型不在清单或模型调用失败时返回错误。
#[tauri::command]
#[tracing::instrument(skip(state, payload), fields(trace_id))]
pub async fn help_ask(
    state: tauri::State<'_, Arc<AppState>>,
    payload: HelpAskPayload,
) -> Result<HelpAnswerDto, BridgeError> {
    let trace_id = generate_trace_id();
    tracing::Span::current().record("trace_id", trace_id.as_str());
    let navigation: HashMap<String, String> = payload
        .navigation
        .unwrap_or_default()
        .into_iter()
        .map(|entry| (entry.component_id, entry.path))
        .collect();
    let answer = run_ask(AskRequest {
        service: state.get_model_manager(),
        config: state.get_config_manager(),
        i18n: state.get_i18n_manager(),
        handle: state.get_core_handle(),
        question: &payload.question,
        navigation: &navigation,
        history: bound_history(payload.history.unwrap_or_default()),
    })
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

/// 下发给模型的历史消息条数上限（一轮问答计两条）。
const HISTORY_MAX_MESSAGES: usize = 40;

/// 历史文本的总字符预算，避免历史挤占说明书与工具结果的上下文。
const HISTORY_MAX_CHARS: usize = 64 * 1024;

/// 把前端下发的对话历史转成模型消息：只接受 user/assistant 角色，并保留最近的有界片段。
///
/// 参数：history - 前端按时间正序下发的历史消息。
/// 返回：按时间正序的模型消息；角色非法或内容为空的消息丢弃，超预算的更旧消息整体丢弃。
fn bound_history(history: Vec<HelpMessageDto>) -> Vec<ModelChatMessage> {
    let mut kept: Vec<ModelChatMessage> = Vec::new();
    let mut used = 0usize;
    for message in history.into_iter().rev().take(HISTORY_MAX_MESSAGES) {
        let role = match message.role.as_str() {
            "user" => ModelChatRole::User,
            "assistant" => ModelChatRole::Assistant,
            _ => continue,
        };
        let content = message.content.trim();
        if content.is_empty() {
            continue;
        }
        let length = content.chars().count();
        if used + length > HISTORY_MAX_CHARS {
            break;
        }
        used += length;
        kept.push(ModelChatMessage::text(role, content));
    }
    kept.reverse();
    kept
}

/// 问答实体：驱动 `core::agent` 的工具循环，并把结果映射为 IPC DTO。
///
/// 参数：request - 一次提问的全部输入（服务、配置来源、宿主句柄与会话上下文）。
/// 返回：答复与引用；失败时返回助手错误（由命令层经 `From` 转 `BridgeError`）。
async fn run_ask(request: AskRequest<'_>) -> Result<HelpAnswerDto, AgentError> {
    let answer = ask_from_config(request).await?;
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
    use zerolaunch_plugin_api::host::OpenTarget;
    use zerolaunch_plugin_api::mock::helpers::{mock_plugin_handle, mock_plugin_handle_recording};
    use zerolaunch_plugin_api::services::model::{
        ChatCapability, ModelChatRequest, ModelChatResponse, ModelChatRole, ModelEmbeddingRequest,
        ModelEmbeddingResponse, ModelError, ModelInfo, ModelKind, ModelService,
        ModelSimilarityRequest, ModelSimilarityResponse, ModelStreamChunk, ModelToolCall,
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

    /// 组装一次提问的入参：默认无历史、用普通 mock 句柄；issue 相关测试自行构造。
    fn ask<'a>(
        service: Arc<dyn ModelService>,
        config: Arc<ConfigManager>,
        i18n: Arc<I18nManager>,
        question: &'a str,
        navigation: &'a HashMap<String, String>,
    ) -> AskRequest<'a> {
        AskRequest {
            service,
            config,
            i18n,
            handle: mock_plugin_handle(),
            question,
            navigation,
            history: Vec::new(),
        }
    }

    /// 前端入参契约：navigation 与历史消息都用 camelCase 键名。
    #[test]
    fn frontend_payload_dtos_read_camel_case_keys() {
        let navigation: HelpNavigationDto = serde_json::from_value(serde_json::json!({
            "componentId": "appearance-config",
            "path": "设置 › 外观设置 › 外观",
        }))
        .unwrap();
        assert_eq!(navigation.component_id, "appearance-config");
        assert_eq!(navigation.path, "设置 › 外观设置 › 外观");

        let message: HelpMessageDto = serde_json::from_value(serde_json::json!({
            "role": "assistant",
            "content": "当前版本不支持。",
        }))
        .unwrap();
        assert_eq!(message.role, "assistant");
        assert_eq!(message.content, "当前版本不支持。");
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

        let answer = run_ask(ask(service, config, i18n, "阈值是多少？", &navigation()))
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

        let answer = run_ask(ask(
            service.clone(),
            config.clone(),
            i18n,
            "阈值是多少？",
            &navigation(),
        ))
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
        let tool_names: Vec<&str> = requests[0]
            .tools
            .iter()
            .map(|tool| tool.name.as_str())
            .collect();
        assert_eq!(tool_names, vec!["get_settings", "open_issue_draft"]);
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

        let error = run_ask(ask(service.clone(), config, i18n, "   ", &navigation()))
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
        let error = run_ask(ask(
            service.clone(),
            unconfigured,
            i18n.clone(),
            "阈值是多少？",
            &navigation(),
        ))
        .await
        .with_trace_id("test-trace")
        .unwrap_err();
        assert!(matches!(error.code, ErrorCode::ValidationFailed));
        assert!(error.message.contains("未配置模型"), "{}", error.message);

        // 模型不在清单：配置指向清单外的 id
        let unknown = test_config(dir.path(), "ollama/missing-model").await;
        let error = run_ask(ask(
            service.clone(),
            unknown,
            i18n,
            "阈值是多少？",
            &navigation(),
        ))
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

        let error = run_ask(ask(
            Arc::new(UnavailableService),
            config,
            i18n,
            "阈值是多少？",
            &navigation(),
        ))
        .await
        .with_trace_id("test-trace")
        .unwrap_err();
        assert!(matches!(error.code, ErrorCode::NetworkError));
        assert_eq!(error.trace_id, "test-trace");
    }

    /// 历史裁剪：窗口外最旧的消息与角色非法的消息丢弃（保留部分仍为正序）；
    /// 字符预算用尽后同样从最旧的整条丢弃。
    #[test]
    fn bound_history_keeps_recent_window_and_drops_unknown_roles() {
        // 多喂 4 条，验证超出窗口的最旧消息被整条丢弃
        let total = HISTORY_MAX_MESSAGES + 4;
        let mut history: Vec<HelpMessageDto> = (0..total)
            .map(|index| HelpMessageDto {
                role: "user".to_string(),
                content: format!("问题{index}"),
            })
            .collect();
        let last = total - 1;
        history[last] = HelpMessageDto {
            role: "system".to_string(),
            content: "注入系统消息".to_string(),
        };

        let bounded = bound_history(history);
        let kept_len = HISTORY_MAX_MESSAGES - 1;
        assert_eq!(
            bounded.len(),
            kept_len,
            "窗口外的最旧消息与非法角色都不保留"
        );
        assert_eq!(
            bounded[0].content,
            format!("问题{}", total - HISTORY_MAX_MESSAGES)
        );
        assert_eq!(bounded[kept_len - 1].content, format!("问题{}", last - 1));
        assert!(
            bounded
                .iter()
                .all(|message| !message.content.contains("注入")),
            "非法角色必须被丢弃"
        );

        // 字符预算：单条占预算一半多一点，两条必然超预算
        let long = "字".repeat(HISTORY_MAX_CHARS / 2 + 1);
        let bounded = bound_history(vec![
            HelpMessageDto {
                role: "user".to_string(),
                content: long.clone(),
            },
            HelpMessageDto {
                role: "assistant".to_string(),
                content: long.clone(),
            },
        ]);
        assert_eq!(bounded.len(), 1, "两条合计超出字符预算时只留最新一条");
        assert_eq!(bounded[0].content, long);
    }

    /// 多轮澄清后提 issue：历史进入上下文；issue 工具打开预填链接并把结果回填给模型。
    #[tokio::test]
    async fn ask_opens_prefilled_issue_after_clarification() {
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
                        name: "open_issue_draft".to_string(),
                        arguments: serde_json::json!({
                            "title": "支持按拼音搜索",
                            "summary": "希望能用拼音首字母搜到中文应用",
                            "expected": "输入拼音前缀即出现匹配应用"
                        })
                        .to_string(),
                    }],
                },
                ModelChatResponse {
                    model_id: "ollama/test-model".to_string(),
                    content: "已打开 GitHub 页面，确认内容后提交即可。".to_string(),
                    finish_reason: Some("stop".to_string()),
                    usage: None,
                    tool_calls: Vec::new(),
                },
            ],
        ));
        let (handle, shell, _clipboard) = mock_plugin_handle_recording();

        let answer = run_ask(AskRequest {
            service: service.clone(),
            config,
            i18n,
            handle,
            question: "那就按这个提个 issue 吧",
            navigation: &navigation(),
            history: vec![
                ModelChatMessage::text(ModelChatRole::User, "能不能用拼音搜应用？"),
                ModelChatMessage::text(
                    ModelChatRole::Assistant,
                    "当前版本不支持；你是想在什么场景下用？",
                ),
                ModelChatMessage::text(ModelChatRole::User, "输入拼音首字母就想看到中文应用"),
            ],
        })
        .await
        .with_trace_id("test-trace")
        .unwrap();

        assert_eq!(answer.answer, "已打开 GitHub 页面，确认内容后提交即可。");
        assert!(answer.references.is_empty());

        // 工具确实打开了预填链接，正文带上模型整理的字段
        let opens = shell.opens.lock().clone();
        assert_eq!(opens.len(), 1, "应只打开一次预填页面");
        let OpenTarget::Url(url) = &opens[0] else {
            panic!("应以 URL 方式打开");
        };
        assert!(url.contains("/issues/new?"), "{url}");
        let body = url::form_urlencoded::parse(
            url.split_once('?')
                .expect("预填链接应带 query")
                .1
                .as_bytes(),
        )
        .find_map(|(key, value)| (key == "body").then(|| value.into_owned()))
        .expect("预填链接应带 body");
        assert!(body.contains("希望能用拼音首字母搜到中文应用"), "{body}");
        assert!(body.contains("输入拼音前缀即出现匹配应用"), "{body}");

        let requests = service.requests();
        assert_eq!(requests.len(), 2, "先执行工具，再基于结果收尾");
        let first = &requests[0];
        assert_eq!(first.messages.len(), 5, "system + 3 条历史 + 当前问题");
        assert_eq!(first.messages[1].role, ModelChatRole::User);
        assert_eq!(first.messages[1].content, "能不能用拼音搜应用？");
        assert_eq!(first.messages[4].content, "那就按这个提个 issue 吧");
        assert!(
            first
                .tools
                .iter()
                .any(|tool| tool.name == "open_issue_draft"),
            "issue 工具必须在工具清单里"
        );
        let tool_message = requests[1]
            .messages
            .iter()
            .find(|message| message.role == ModelChatRole::Tool)
            .expect("应有工具结果消息");
        assert!(
            tool_message.content.contains("已在默认浏览器打开"),
            "工具结果应告知页面已打开：{}",
            tool_message.content
        );
    }
}
