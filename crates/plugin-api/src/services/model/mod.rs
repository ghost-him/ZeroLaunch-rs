//! 模型服务域：宿主统一提供 chat/embedding 模型调用能力。
//!
//! 提供方（实现 `ModelProvider` 的内置后端或第三方插件）向宿主注册模型清单；
//! 消费者（内置组件经 PluginHandle、第三方插件经 `host/model.*` RPC）通过
//! `ModelService` 按 model_id 调用；宿主 `ModelManager` 聚合清单并按 model_id
//! `{provider}/` 前缀路由到对应提供方。

mod provider;
mod service;
mod types;

pub use provider::ModelProvider;
pub use service::ModelService;
pub use types::{
    ChatCapability, EmbeddingCapability, EmbeddingTemplateArgs, ModelChatMessage, ModelChatRequest,
    ModelChatResponse, ModelChatRole, ModelEmbeddingRequest, ModelEmbeddingResponse, ModelError,
    ModelInfo, ModelKind, ModelSimilarity, ModelSimilarityRequest, ModelSimilarityResponse,
    ModelStreamChunk, ModelTokenUsage, ModelTool, ModelToolCall, SemanticTask,
};
