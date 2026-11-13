//! Ollama 模型提供方：经 ollama-rs 调用本地 Ollama 服务
//! （原生 /api/chat、/api/embeddings、本地模型枚举）。

use super::compose::{compose_embedding_texts, require_embedding_capability};
use super::settings::{ChatModelConfig, ModelEntryConfig, ModelOllamaSettings};
use async_trait::async_trait;
use futures_util::StreamExt;
use ollama_rs::generation::chat::request::ChatMessageRequest;
use ollama_rs::generation::chat::ChatMessage;
use ollama_rs::generation::embeddings::request::{EmbeddingsInput, GenerateEmbeddingsRequest};
use ollama_rs::generation::parameters::ThinkType;
use ollama_rs::generation::tools::{ToolCall, ToolCallFunction, ToolInfo};
use ollama_rs::models::ModelOptions;
use ollama_rs::Ollama;
use std::collections::HashSet;
use std::pin::pin;
use tokio::sync::mpsc;
use zerolaunch_plugin_api::services::model::{
    ChatCapability, EmbeddingCapability, ModelChatRequest, ModelChatResponse, ModelChatRole,
    ModelEmbeddingRequest, ModelEmbeddingResponse, ModelError, ModelInfo, ModelKind, ModelProvider,
    ModelStreamChunk, ModelTool, ModelToolCall,
};

/// 提供方标识：与 model_id 的 `ollama/` 前缀一致。
pub const PROVIDER_ID: &str = "ollama";

/// 总超时：模型生成/embedding/本地模型枚举单次调用上限（reqwest 无默认总超时，
/// 误配 base_url 或服务挂起时会永久阻塞；外层 timeout 兜底）。
const MODEL_CALL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);

/// 截取 model_id 的模型名部分（去掉 `provider/` 前缀）。
fn short_model(model_id: &str) -> &str {
    model_id.split_once('/').map(|(_, m)| m).unwrap_or(model_id)
}

/// 按模型名查找配置条目。
fn entry_for<'a>(
    settings: &'a ModelOllamaSettings,
    model_id: &str,
) -> Option<&'a ModelEntryConfig> {
    let name = short_model(model_id);
    settings.models.iter().find(|entry| entry.name() == name)
}

/// 提取 chat 配置，缺少 chat 条目时返回 None。
fn chat_config(entry: Option<&ModelEntryConfig>) -> Option<&ChatModelConfig> {
    match entry {
        Some(ModelEntryConfig::Chat { config, .. }) => Some(config),
        _ => None,
    }
}

/// 按消息内容和最终输出预算计算本次 Ollama 请求的上下文窗口。
fn dynamic_num_ctx(req: &ModelChatRequest, max_tokens: u32, context_length: u32) -> u64 {
    const CONTEXT_MARGIN_TOKENS: u64 = 256;
    const MIN_CONTEXT_TOKENS: u64 = 512;
    let input_bytes = req
        .messages
        .iter()
        .map(|message| message.content.len() as u64)
        .sum::<u64>();
    let requested = input_bytes
        .saturating_add(max_tokens as u64)
        .saturating_add(CONTEXT_MARGIN_TOKENS)
        .max(MIN_CONTEXT_TOKENS);
    if context_length > 0 {
        requested.min(context_length as u64)
    } else {
        requested
    }
}

/// 工具参数文本转 JSON 值：优先按 JSON 解析，非法时原样包成字符串交给模型自纠。
fn arguments_to_value(arguments: &str) -> serde_json::Value {
    serde_json::from_str(arguments)
        .unwrap_or_else(|_| serde_json::Value::String(arguments.to_string()))
}

/// 将统一工具定义转换为 Ollama 工具描述。
///
/// 经 serde 构造 `ToolInfo`：其 `parameters` 为 schemars 的 `Schema`（未由 ollama-rs
/// 重导出），走 JSON 同时也锁定了与该 crate 一致的线格式。
fn to_ollama_tools(tools: &[ModelTool]) -> Result<Vec<ToolInfo>, ModelError> {
    tools
        .iter()
        .map(|tool| {
            serde_json::from_value(serde_json::json!({
                "type": "function",
                "function": {
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": tool.parameters,
                },
            }))
            .map_err(|e| ModelError::InvalidRequest(format!("工具 {} 定义非法: {e}", tool.name)))
        })
        .collect()
}

/// 构造 Ollama chat 请求：请求显式值优先，其余使用模型或通用默认值。
fn build_chat_request(
    req: &ModelChatRequest,
    entry: Option<&ModelEntryConfig>,
) -> Result<ChatMessageRequest, ModelError> {
    let messages: Vec<ChatMessage> = req
        .messages
        .iter()
        .map(|m| match m.role {
            ModelChatRole::System => ChatMessage::system(m.content.clone()),
            ModelChatRole::User => ChatMessage::user(m.content.clone()),
            ModelChatRole::Assistant => {
                let mut message = ChatMessage::assistant(m.content.clone());
                message.tool_calls = m
                    .tool_calls
                    .iter()
                    .map(|call| ToolCall {
                        function: ToolCallFunction {
                            name: call.name.clone(),
                            arguments: arguments_to_value(&call.arguments),
                        },
                    })
                    .collect();
                message
            }
            ModelChatRole::Tool => ChatMessage::tool(m.content.clone()),
        })
        .collect();
    let defaults = chat_config(entry).cloned().unwrap_or_default();
    let temperature = req.temperature.unwrap_or(defaults.temperature);
    let max_tokens = req.max_tokens.unwrap_or(defaults.max_tokens);
    let num_ctx = dynamic_num_ctx(req, max_tokens, defaults.context_length);
    let mut options = ModelOptions::default();
    options = options
        .temperature(temperature)
        .top_p(defaults.top_p)
        .num_ctx(num_ctx)
        .num_predict(max_tokens.min(i32::MAX as u32) as i32);
    let mut request =
        ChatMessageRequest::new(short_model(&req.model_id).to_string(), messages).options(options);
    let reasoning_effort = req
        .reasoning_effort
        .clone()
        .unwrap_or(defaults.reasoning_effort);
    if reasoning_effort != "none" {
        if let Some(config) = chat_config(entry) {
            if !config.capabilities.contains(&ChatCapability::Reasoning) {
                return Err(ModelError::InvalidRequest(
                    "模型未声明支持 reasoning 能力".to_string(),
                ));
            }
        }
    }
    let think = match reasoning_effort.as_str() {
        "none" => ThinkType::False,
        "low" => ThinkType::Low,
        "medium" => ThinkType::Medium,
        "high" => ThinkType::High,
        other => {
            return Err(ModelError::InvalidRequest(format!(
                "无效的 reasoning_effort: {other}"
            )));
        }
    };
    request = request.think(think);
    if !req.tools.is_empty() {
        if let Some(config) = chat_config(entry) {
            if !config.capabilities.contains(&ChatCapability::Tools) {
                return Err(ModelError::InvalidRequest(
                    "模型未声明支持 tools 能力".to_string(),
                ));
            }
        }
        request = request.tools(to_ollama_tools(&req.tools)?);
    }
    Ok(request)
}

/// Ollama 提供方：客户端按调用时构建（base_url 读配置），模型清单来自本地枚举 + 配置声明。
pub struct OllamaProvider {
    settings: ModelOllamaSettings,
}

impl OllamaProvider {
    /// 创建提供方实例。
    pub fn new(settings: ModelOllamaSettings) -> Self {
        Self { settings }
    }

    /// 按当前配置构建 Ollama 客户端（base_url 非法时返回错误）。
    fn client(&self) -> Result<Ollama, ModelError> {
        Ollama::try_new(self.settings.base_url.trim())
            .map_err(|e| ModelError::ProviderUnavailable(e.to_string()))
    }
}

#[async_trait]
impl ModelProvider for OllamaProvider {
    fn provider_id(&self) -> String {
        PROVIDER_ID.to_string()
    }
    /// 返回 endpoint 与模型配置组成的 embedding 缓存命名空间。
    fn cache_namespace(&self) -> String {
        // 仅含 provider + base_url：请求参数已参与 compute_key，全量 models JSON 掺入
        // 会让无关配置改动清空整个 embedding 缓存。
        format!("{}:{}", PROVIDER_ID, self.settings.base_url)
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, ModelError> {
        let local = tokio::time::timeout(MODEL_CALL_TIMEOUT, self.client()?.list_local_models())
            .await
            .map_err(|_| ModelError::ProviderUnavailable("模型清单获取超时".to_string()))?
            .map_err(|e| ModelError::ProviderUnavailable(e.to_string()))?;
        let mut models = Vec::new();
        // 空清单 = 全部本地模型按 chat 列出（既有语义）；有声明时按条目 kind 归类，
        // embedding 声明优先（同名模型按 embedding 列出）。
        let empty = self.settings.models.is_empty();
        let configured = self
            .settings
            .models
            .iter()
            .filter(|entry| !entry.name().is_empty());
        let chat_names: HashSet<&str> = configured
            .clone()
            .filter_map(|entry| match entry {
                ModelEntryConfig::Chat { name, .. } => Some(name.as_str()),
                ModelEntryConfig::Embedding { .. } => None,
            })
            .collect();
        let embedding_entries: Vec<&ModelEntryConfig> = configured
            .filter(|entry| matches!(entry, ModelEntryConfig::Embedding { .. }))
            .collect();

        for m in &local {
            let name = m.name.clone();
            if let Some(ModelEntryConfig::Embedding { config, .. }) =
                embedding_entries.iter().find(|entry| entry.name() == name)
            {
                models.push(ModelInfo {
                    model_id: format!("{PROVIDER_ID}/{name}"),
                    name: name.clone(),
                    kind: ModelKind::Embedding {
                        context_window: (config.context_length > 0)
                            .then_some(config.context_length),
                        dimensions: (config.dimensions > 0).then_some(config.dimensions),
                        similarity: config.similarity,
                        capabilities: config.capabilities.clone(),
                    },
                    provider: PROVIDER_ID.to_string(),
                });
            } else if empty || chat_names.contains(name.as_str()) {
                let (context_window, capabilities) = self
                    .settings
                    .models
                    .iter()
                    .find_map(|entry| match entry {
                        ModelEntryConfig::Chat {
                            name: configured_name,
                            config,
                        } if configured_name == &name => Some((
                            (config.context_length > 0).then_some(config.context_length),
                            config.capabilities.clone(),
                        )),
                        _ => None,
                    })
                    // 未在配置里声明的模型按主流 chat 模型的默认能力声明（tools + reasoning）；
                    // provider 的能力闸门只在条目明确声明时校验，此处仅影响模型清单展示。
                    .unwrap_or((None, vec![ChatCapability::Reasoning, ChatCapability::Tools]));
                models.push(ModelInfo {
                    model_id: format!("{PROVIDER_ID}/{name}"),
                    name,
                    kind: ModelKind::Chat {
                        context_window,
                        max_output: None,
                        supports_stream: true,
                        capabilities,
                    },
                    provider: PROVIDER_ID.to_string(),
                });
            }
        }
        Ok(models)
    }

    async fn chat(&self, req: ModelChatRequest) -> Result<ModelChatResponse, ModelError> {
        let entry = entry_for(&self.settings, &req.model_id);
        let request = build_chat_request(&req, entry)?;
        let resp = tokio::time::timeout(
            MODEL_CALL_TIMEOUT,
            self.client()?.send_chat_messages(request),
        )
        .await
        .map_err(|_| ModelError::Transport("模型调用超时".to_string()))?
        .map_err(|e| ModelError::Transport(e.to_string()))?;
        Ok(ModelChatResponse {
            model_id: req.model_id,
            content: resp.message.content,
            finish_reason: None,
            usage: None,
            tool_calls: resp
                .message
                .tool_calls
                .into_iter()
                .map(|call| ModelToolCall {
                    // Ollama 不返回调用 id，工具结果按单轮配对即可
                    id: String::new(),
                    name: call.function.name,
                    arguments: call.function.arguments.to_string(),
                })
                .collect(),
        })
    }

    async fn stream_chat(
        &self,
        req: ModelChatRequest,
        tx: mpsc::Sender<ModelStreamChunk>,
    ) -> Result<(), ModelError> {
        let entry = entry_for(&self.settings, &req.model_id);
        let request = build_chat_request(&req, entry)?;
        let stream = tokio::time::timeout(
            MODEL_CALL_TIMEOUT,
            self.client()?.send_chat_messages_stream(request),
        )
        .await
        .map_err(|_| ModelError::Transport("模型调用超时".to_string()))?
        .map_err(|e| ModelError::Transport(e.to_string()))?;
        let mut stream = pin!(stream);

        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(chunk) => {
                    tx.send(ModelStreamChunk::Delta(chunk.message.content))
                        .await
                        .map_err(|_| ModelError::Transport("流通道已关闭".to_string()))?;
                }
                Err(_) => {
                    let _ = tx
                        .send(ModelStreamChunk::Error(
                            "Ollama 流式输出传输失败".to_string(),
                        ))
                        .await;
                    return Err(ModelError::Transport("Ollama 流式输出传输失败".to_string()));
                }
            }
        }
        let _ = tx
            .send(ModelStreamChunk::Done {
                finish_reason: None,
            })
            .await;
        Ok(())
    }

    async fn embedding(
        &self,
        req: ModelEmbeddingRequest,
    ) -> Result<ModelEmbeddingResponse, ModelError> {
        let entry = entry_for(&self.settings, &req.model_id);
        let embedding_config = entry.and_then(|value| match value {
            ModelEntryConfig::Embedding { config, .. } => Some(config),
            ModelEntryConfig::Chat { .. } => None,
        });
        let texts = compose_embedding_texts(
            &req.input,
            req.task_type,
            &req.model_id,
            req.template_args.as_deref(),
        )?;
        let mut request = GenerateEmbeddingsRequest::new(
            short_model(&req.model_id).to_string(),
            EmbeddingsInput::Multiple(texts),
        );
        if let Some(dimensions) = req.dimensions {
            if let Some(config) = embedding_config {
                require_embedding_capability(
                    &config.capabilities,
                    EmbeddingCapability::OutputDimensions,
                    "outputDimensions",
                )?;
            }
            request.dimensions = Some(dimensions);
        }
        let resp = tokio::time::timeout(
            MODEL_CALL_TIMEOUT,
            self.client()?.generate_embeddings(request),
        )
        .await
        .map_err(|_| ModelError::Transport("模型调用超时".to_string()))?
        .map_err(|e| ModelError::Transport(e.to_string()))?;
        let dimensions = resp.embeddings.first().map(|v| v.len()).unwrap_or(0);
        Ok(ModelEmbeddingResponse {
            model_id: req.model_id,
            dimensions: dimensions as u32,
            vectors: resp.embeddings,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use zerolaunch_plugin_api::services::model::ModelChatMessage;

    /// 工具定义转换为 Ollama 线格式：参数 schema 经 schemars 往返后保留 properties/required。
    #[test]
    fn to_ollama_tools_preserves_parameters_schema() {
        let tools = vec![ModelTool {
            name: "get_settings".to_string(),
            description: "读取设置".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": { "component_id": { "type": "string", "description": "组件 id" } },
                "required": ["component_id"],
            }),
        }];
        let converted = to_ollama_tools(&tools).unwrap();
        let json = serde_json::to_value(&converted[0]).unwrap();
        assert_eq!(json["type"], "function");
        assert_eq!(json["function"]["name"], "get_settings");
        assert_eq!(json["function"]["description"], "读取设置");
        assert_eq!(json["function"]["parameters"]["type"], "object");
        assert_eq!(
            json["function"]["parameters"]["properties"]["component_id"]["type"],
            "string"
        );
        assert_eq!(
            json["function"]["parameters"]["required"][0],
            "component_id"
        );
    }

    /// 非法参数 schema 直接拒绝，不静默降级为无参工具。
    #[test]
    fn to_ollama_tools_rejects_invalid_parameters() {
        let tools = vec![ModelTool {
            name: "broken".to_string(),
            description: String::new(),
            parameters: serde_json::json!("not-a-schema"),
        }];
        assert!(to_ollama_tools(&tools).is_err());
    }

    /// 助手工具调用与工具结果映射为 Ollama 的 tool_calls 与 tool 角色。
    #[test]
    fn build_chat_request_maps_tool_messages() {
        let request = ModelChatRequest {
            model_id: "ollama/test".to_string(),
            messages: vec![
                ModelChatMessage {
                    role: ModelChatRole::Assistant,
                    content: String::new(),
                    tool_calls: vec![ModelToolCall {
                        id: "call-1".to_string(),
                        name: "get_settings".to_string(),
                        arguments: r#"{"component_id":"demo"}"#.to_string(),
                    }],
                    tool_call_id: None,
                },
                ModelChatMessage {
                    role: ModelChatRole::Tool,
                    content: "结果".to_string(),
                    tool_calls: Vec::new(),
                    tool_call_id: Some("call-1".to_string()),
                },
            ],
            temperature: None,
            max_tokens: None,
            reasoning_effort: None,
            tools: Vec::new(),
        };
        let built = build_chat_request(&request, None).unwrap();
        assert_eq!(
            serde_json::to_value(&built.messages[0].role).unwrap(),
            serde_json::json!("assistant")
        );
        assert_eq!(
            built.messages[0].tool_calls[0].function.name,
            "get_settings"
        );
        assert_eq!(
            built.messages[0].tool_calls[0].function.arguments,
            serde_json::json!({ "component_id": "demo" })
        );
        assert_eq!(
            serde_json::to_value(&built.messages[1].role).unwrap(),
            serde_json::json!("tool")
        );
        assert_eq!(built.messages[1].content, "结果");
    }

    /// 工具参数文本按 JSON 解析；非法文本原样包成字符串交给模型自纠。
    #[test]
    fn arguments_to_value_parses_json_and_keeps_invalid_text() {
        assert_eq!(
            arguments_to_value(r#"{"a":1}"#),
            serde_json::json!({ "a": 1 })
        );
        assert_eq!(arguments_to_value("abc"), serde_json::json!("abc"));
    }

    #[test]
    fn entry_for_matches_short_model_name() {
        let settings = ModelOllamaSettings {
            base_url: "http://localhost:11434".to_string(),
            models: vec![ModelEntryConfig::chat("qwen3:8b")],
        };
        assert!(entry_for(&settings, "ollama/qwen3:8b").is_some());
        assert!(entry_for(&settings, "ollama/other").is_none());
    }
    /// 验证动态窗口包含最终输出预算，并受模型最大上下文限制。
    #[test]
    fn dynamic_num_ctx_uses_request_size_and_model_limit() {
        let request = ModelChatRequest {
            model_id: "ollama/test".to_string(),
            messages: vec![ModelChatMessage {
                role: ModelChatRole::User,
                content: "a".repeat(5000),
                tool_calls: Vec::new(),
                tool_call_id: None,
            }],
            temperature: None,
            max_tokens: None,
            reasoning_effort: None,
            tools: Vec::new(),
        };
        let config = ChatModelConfig {
            context_length: 4096,
            ..Default::default()
        };

        assert_eq!(
            dynamic_num_ctx(&request, config.max_tokens, config.context_length),
            4096
        );

        let short_request = ModelChatRequest {
            messages: vec![ModelChatMessage {
                role: ModelChatRole::User,
                content: "abc".to_string(),
                tool_calls: Vec::new(),
                tool_call_id: None,
            }],
            ..request
        };
        let uncapped_config = ChatModelConfig {
            context_length: 0,
            ..Default::default()
        };
        assert_eq!(
            dynamic_num_ctx(
                &short_request,
                uncapped_config.max_tokens,
                uncapped_config.context_length,
            ),
            2307
        );

        let explicit_request = ModelChatRequest {
            max_tokens: Some(8192),
            ..short_request
        };
        assert_eq!(dynamic_num_ctx(&explicit_request, 8192, 20000), 8451);
    }
}
