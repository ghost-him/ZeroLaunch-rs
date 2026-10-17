//! 宿主模型管理核心：聚合模型提供方，按 model_id 前缀路由调用。
//!
//! 内置提供方（OpenAI 兼容 / Ollama）由连接配置构建；未来第三方插件提供方
//! 仍通过公共 `ModelProvider` SPI 接入。调用方经 `ModelService` 使用聚合模型目录。

mod compose;
mod embedding_cache;
pub(crate) mod model_profiles;
mod ollama_provider;
mod openai_compatible_provider;
mod settings;

pub use embedding_cache::EmbeddingCache;
pub use settings::{
    ChatModelConfig, EmbeddingModelConfig, ModelEntryConfig, ModelOllamaSettings,
    ModelOpenAiSettings,
};

use self::compose::require_embedding_capability;
use crate::core::config::{ConfigEvent, ConfigManager};
use async_trait::async_trait;
use dashmap::DashMap;
use parking_lot::RwLock;
use rayon::prelude::*;
use std::sync::Arc;
use tokio::sync::mpsc;
use zerolaunch_plugin_api::services::model::{
    EmbeddingCapability, EmbeddingTemplateArgs, ModelChatRequest, ModelChatResponse,
    ModelEmbeddingRequest, ModelEmbeddingResponse, ModelError, ModelInfo, ModelKind, ModelProvider,
    ModelService, ModelSimilarity, ModelSimilarityRequest, ModelSimilarityResponse,
    ModelStreamChunk,
};

/// OpenAI 兼容提供方的配置组件 id。
pub const MODEL_OPENAI_CONFIG_ID: &str = "model-openai-config";
/// Ollama 提供方的配置组件 id。
pub const MODEL_OLLAMA_CONFIG_ID: &str = "model-ollama-config";

/// 模型管理器：聚合提供方清单并按 model_id 前缀路由调用。
pub struct ModelManager {
    providers: DashMap<String, Arc<dyn ModelProvider>>,
    models: RwLock<Vec<ModelInfo>>,
    /// embedding 结果缓存（可选；bootstrap 在 core handle 就绪后注入）
    cache: parking_lot::RwLock<Option<Arc<EmbeddingCache>>>,
}

impl Default for ModelManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelManager {
    /// 创建空注册表（启动后经 register_builtin_providers 填充）。
    pub fn new() -> Self {
        Self {
            providers: DashMap::new(),
            models: RwLock::new(Vec::new()),
            cache: parking_lot::RwLock::new(None),
        }
    }

    /// 注册提供方（同 id 覆盖）。
    pub fn register_provider(&self, provider: Arc<dyn ModelProvider>) {
        self.providers.insert(provider.provider_id(), provider);
    }

    /// 解注册提供方（插件卸载时调用）。
    pub fn unregister_provider(&self, provider_id: &str) {
        self.providers.remove(provider_id);
    }

    /// 注入 embedding 缓存（启动序列中 core handle 就绪后调用）。
    pub fn set_cache(&self, cache: Arc<EmbeddingCache>) {
        *self.cache.write() = Some(cache);
    }

    /// 按 ConfigManager 当前配置重建内置提供方（openai 兼容 / ollama）。
    pub fn register_builtin_providers(&self, cm: &ConfigManager) {
        let openai: ModelOpenAiSettings = cm
            .get_settings(MODEL_OPENAI_CONFIG_ID)
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();
        self.register_provider(Arc::new(
            openai_compatible_provider::OpenAiCompatibleProvider::new(openai),
        ));

        let ollama: ModelOllamaSettings = cm
            .get_settings(MODEL_OLLAMA_CONFIG_ID)
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();
        self.register_provider(Arc::new(ollama_provider::OllamaProvider::new(ollama)));
    }

    /// 并发拉取（join_all）并给每个提供方外层超时，避免单点挂起阻塞整体刷新。
    pub async fn refresh_models(&self) {
        // 先收集 Arc 再并发 await：DashMap 迭代守卫不可跨 await 持有
        let providers: Vec<Arc<dyn ModelProvider>> =
            self.providers.iter().map(|e| e.clone()).collect();
        let all: Vec<Vec<ModelInfo>> =
            futures_util::future::join_all(providers.into_iter().map(|provider| async move {
                match tokio::time::timeout(
                    std::time::Duration::from_secs(30),
                    provider.list_models(),
                )
                .await
                {
                    Ok(Ok(models)) => models,
                    Ok(Err(e)) => {
                        tracing::warn!("模型提供方 {} 清单获取失败: {}", provider.provider_id(), e);
                        Vec::new()
                    }
                    Err(_) => {
                        tracing::warn!("模型提供方 {} 清单获取超时", provider.provider_id());
                        Vec::new()
                    }
                }
            }))
            .await;
        *self.models.write() = all.into_iter().flatten().collect();
    }

    /// 内置模型配置变更时重建提供方并刷新清单。
    pub async fn handle_config_event(&self, cm: &ConfigManager, event: &ConfigEvent) {
        let ConfigEvent::SettingsChanged { component_id, .. } = event else {
            return;
        };
        if component_id == MODEL_OPENAI_CONFIG_ID || component_id == MODEL_OLLAMA_CONFIG_ID {
            self.register_builtin_providers(cm);
            self.refresh_models().await;
        }
    }

    /// 按 model_id 的 `{provider}/` 前缀路由到提供方。
    fn route(&self, model_id: &str) -> Result<Arc<dyn ModelProvider>, ModelError> {
        let (provider_id, _) = model_id.split_once('/').ok_or_else(|| {
            ModelError::InvalidRequest(format!("模型 id 缺少提供方前缀: {model_id}"))
        })?;
        self.providers
            .get(provider_id)
            .map(|e| e.clone())
            .ok_or_else(|| ModelError::ModelNotFound(model_id.to_string()))
    }
}

#[async_trait]
impl ModelService for ModelManager {
    fn list_models(&self) -> Vec<ModelInfo> {
        self.models.read().clone()
    }

    fn model_info(&self, model_id: &str) -> Option<ModelInfo> {
        self.models
            .read()
            .iter()
            .find(|m| m.model_id == model_id)
            .cloned()
    }

    async fn chat(&self, req: ModelChatRequest) -> Result<ModelChatResponse, ModelError> {
        let provider = self.route(&req.model_id)?;
        match self.model_info(&req.model_id).map(|info| info.kind) {
            Some(ModelKind::Chat { .. }) => provider.chat(req).await,
            Some(ModelKind::Embedding { .. }) => Err(ModelError::NotSupported),
            None => Err(ModelError::ModelNotFound(req.model_id)),
        }
    }

    async fn stream_chat(
        &self,
        req: ModelChatRequest,
        tx: mpsc::Sender<ModelStreamChunk>,
    ) -> Result<(), ModelError> {
        let provider = self.route(&req.model_id)?;
        match self.model_info(&req.model_id).map(|info| info.kind) {
            Some(ModelKind::Chat {
                supports_stream: true,
                ..
            }) => provider.stream_chat(req, tx).await,
            Some(ModelKind::Chat { .. }) => Err(ModelError::NotSupported),
            Some(ModelKind::Embedding { .. }) => Err(ModelError::NotSupported),
            None => Err(ModelError::ModelNotFound(req.model_id)),
        }
    }

    async fn embedding(
        &self,
        req: ModelEmbeddingRequest,
    ) -> Result<ModelEmbeddingResponse, ModelError> {
        let provider = self.route(&req.model_id)?;
        let model_info = self
            .model_info(&req.model_id)
            .ok_or_else(|| ModelError::ModelNotFound(req.model_id.clone()))?;
        let ModelKind::Embedding {
            capabilities,
            dimensions: expected_dimensions,
            ..
        } = &model_info.kind
        else {
            return Err(ModelError::NotSupported);
        };
        if req.dimensions.is_some() {
            require_embedding_capability(
                capabilities,
                EmbeddingCapability::OutputDimensions,
                "outputDimensions",
            )?;
        }
        let expected_dimensions = *expected_dimensions;
        let cache = self.cache.read().clone();
        // 单文本粒度拆分：命中的直接取用，未命中的合并为一次 provider 批量调用。
        let cache_namespace = provider.cache_namespace();
        struct Miss {
            idx: usize,
            text: String,
            args: Option<EmbeddingTemplateArgs>,
            key: [u8; 32],
        }
        let mut cached: Vec<(usize, Arc<Vec<f32>>)> = Vec::new();
        let mut misses: Vec<Miss> = Vec::new();
        if let Some(cache) = &cache {
            for (idx, text) in req.input.iter().enumerate() {
                let args = req
                    .template_args
                    .as_ref()
                    .and_then(|args| args.get(idx).cloned());
                let key = embedding_cache::compute_key(
                    &cache_namespace,
                    &ModelEmbeddingRequest {
                        model_id: req.model_id.clone(),
                        input: vec![text.clone()],
                        template_args: args.clone().map(|a| vec![a]),
                        task_type: req.task_type,
                        dimensions: req.dimensions,
                    },
                );
                match cache.get(&key).await {
                    Some(vec) => {
                        if expected_dimensions
                            .is_some_and(|dimensions| vec.len() as u32 != dimensions)
                        {
                            return Err(ModelError::Transport(
                                "缓存向量维度与模型元数据不匹配".to_string(),
                            ));
                        }
                        cached.push((idx, vec));
                    }
                    None => misses.push(Miss {
                        idx,
                        text: text.clone(),
                        args,
                        key,
                    }),
                }
            }
        } else {
            misses = req
                .input
                .iter()
                .enumerate()
                .map(|(idx, text)| Miss {
                    idx,
                    text: text.clone(),
                    args: req
                        .template_args
                        .as_ref()
                        .and_then(|args| args.get(idx).cloned()),
                    key: [0; 32],
                })
                .collect();
        }

        let mut ordered: Vec<Vec<f32>> = vec![Vec::new(); req.input.len()];
        for (idx, vec) in &cached {
            ordered[*idx] = vec.as_ref().clone();
        }
        if !misses.is_empty() {
            let sub = ModelEmbeddingRequest {
                model_id: req.model_id.clone(),
                input: misses.iter().map(|m| m.text.clone()).collect(),
                template_args: Some(
                    misses
                        .iter()
                        .map(|m| m.args.clone().unwrap_or_default())
                        .collect(),
                ),
                task_type: req.task_type,
                dimensions: req.dimensions,
            };
            let resp = provider.embedding(sub).await?;
            if resp.vectors.len() != misses.len() {
                return Err(ModelError::Transport("provider 向量数量不匹配".to_string()));
            }
            let actual_dimensions = resp.vectors.first().map(Vec::len).unwrap_or_default();
            if expected_dimensions.is_some_and(|dimensions| actual_dimensions as u32 != dimensions)
            {
                return Err(ModelError::Transport(
                    "provider 向量维度与模型元数据不匹配".to_string(),
                ));
            }
            if resp
                .vectors
                .iter()
                .any(|vector| vector.len() != actual_dimensions)
            {
                return Err(ModelError::Transport("provider 向量维度不一致".to_string()));
            }
            for (i, miss) in misses.iter().enumerate() {
                let vec = resp.vectors[i].clone();
                ordered[miss.idx] = vec.clone();
                if let Some(cache) = &cache {
                    cache.put(&miss.key, vec.len() as u32, vec).await;
                }
            }
        }

        let dimensions = ordered
            .first()
            .map(|vector| vector.len())
            .unwrap_or_default();
        if ordered.iter().any(|vector| vector.len() != dimensions) {
            return Err(ModelError::Transport("缓存向量维度不一致".to_string()));
        }
        Ok(ModelEmbeddingResponse {
            model_id: req.model_id,
            dimensions: dimensions as u32,
            vectors: ordered,
        })
    }

    /// 查询向量与多个目标向量的相似度：按模型元数据 similarity 公式计算。
    ///
    /// 向量必须同维度（与 query 一致）；计算公式由模型元数据指定，
    /// 目标向量两两独立，用 rayon 并行加速。非 embedding 模型返回 NotSupported。
    async fn similarity(
        &self,
        req: ModelSimilarityRequest,
    ) -> Result<ModelSimilarityResponse, ModelError> {
        let model_info = self
            .model_info(&req.model_id)
            .ok_or_else(|| ModelError::ModelNotFound(req.model_id.clone()))?;
        let ModelKind::Embedding { similarity, .. } = &model_info.kind else {
            return Err(ModelError::NotSupported);
        };
        let similarity = *similarity;
        let query = req.query.clone();
        let dim = query.len();
        if dim == 0 {
            return Err(ModelError::InvalidRequest("查询向量为空".to_string()));
        }
        if req.targets.iter().any(|t| t.len() != dim) {
            return Err(ModelError::InvalidRequest(
                "目标向量维度与查询向量不一致".to_string(),
            ));
        }
        // 相似度计算是 CPU 密集纯函数：移出 async 上下文（spawn_blocking），
        // 避免 rayon par_iter 同步阻塞 tokio worker 线程。
        let targets = req.targets.clone();
        let similarities: Vec<f32> = tauri::async_runtime::spawn_blocking(move || {
            targets
                .par_iter()
                .map(|target| match similarity {
                    ModelSimilarity::Cosine => cosine_similarity(&query, target),
                    ModelSimilarity::DotProduct => dot_product(&query, target),
                    ModelSimilarity::Euclidean => -euclidean_distance(&query, target),
                })
                .collect()
        })
        .await
        .map_err(|e| ModelError::Internal(format!("相似度计算任务失败: {}", e)))?;
        Ok(ModelSimilarityResponse {
            model_id: req.model_id,
            similarities,
        })
    }
}

/// 余弦相似度（真余弦，不要求向量归一化；维度不一致视为不相似 0）。
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    let (dot, na, nb) = a
        .iter()
        .zip(b)
        .fold((0f32, 0f32, 0f32), |(d, x, y), (u, v)| {
            (d + u * v, x + u * u, y + v * v)
        });
    let denom = na.sqrt() * nb.sqrt();
    if denom <= f32::EPSILON || !denom.is_finite() {
        return 0.0;
    }
    dot / denom
}

/// 点积（未归一化向量的相似度）。
fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// 欧氏距离（用于 euclidean 相似度取负）。
fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return f32::MAX;
    }
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| {
            let d = x - y;
            d * d
        })
        .sum::<f32>()
        .sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use zerolaunch_plugin_api::services::model::{
        ChatCapability, EmbeddingCapability, ModelChatMessage, ModelChatRole, ModelKind,
        ModelSimilarity, ModelTokenUsage, SemanticTask,
    };

    /// 记录型桩提供方：按构造参数返回固定模型清单与调用结果。
    struct StubProvider {
        id: &'static str,
        models: Vec<ModelInfo>,
        chat_result: Result<ModelChatResponse, ModelError>,
        embedding_result: Result<ModelEmbeddingResponse, ModelError>,
        chat_calls: Arc<std::sync::atomic::AtomicU32>,
    }

    #[async_trait]
    impl ModelProvider for StubProvider {
        fn provider_id(&self) -> String {
            self.id.to_string()
        }
        async fn list_models(&self) -> Result<Vec<ModelInfo>, ModelError> {
            Ok(self.models.clone())
        }
        async fn chat(&self, req: ModelChatRequest) -> Result<ModelChatResponse, ModelError> {
            self.chat_calls
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            self.chat_result.clone().map(|mut r| {
                r.model_id = req.model_id;
                r
            })
        }
        async fn stream_chat(
            &self,
            _req: ModelChatRequest,
            _tx: mpsc::Sender<ModelStreamChunk>,
        ) -> Result<(), ModelError> {
            Err(ModelError::NotSupported)
        }
        async fn embedding(
            &self,
            req: ModelEmbeddingRequest,
        ) -> Result<ModelEmbeddingResponse, ModelError> {
            self.embedding_result.clone().map(|mut r| {
                r.model_id = req.model_id;
                r
            })
        }
    }

    fn model(id: &str, kind: ModelKind) -> ModelInfo {
        ModelInfo {
            model_id: id.to_string(),
            name: id.to_string(),
            kind,
            provider: id.split('/').next().unwrap().to_string(),
        }
    }

    #[tokio::test]
    async fn list_models_aggregates_all_providers() {
        let registry = ModelManager::new();
        registry.register_provider(Arc::new(StubProvider {
            id: "chat-pro",
            models: vec![model(
                "chat-pro/m1",
                ModelKind::Chat {
                    context_window: None,
                    max_output: None,
                    supports_stream: false,
                    capabilities: vec![ChatCapability::Reasoning],
                },
            )],
            chat_result: Err(ModelError::NotSupported),
            embedding_result: Err(ModelError::NotSupported),
            chat_calls: Arc::new(Default::default()),
        }));
        registry.register_provider(Arc::new(StubProvider {
            id: "emb-pro",
            models: vec![model(
                "emb-pro/e1",
                ModelKind::Embedding {
                    context_window: None,
                    dimensions: None,
                    similarity: ModelSimilarity::Cosine,
                    capabilities: vec![EmbeddingCapability::OutputDimensions],
                },
            )],
            chat_result: Err(ModelError::NotSupported),
            embedding_result: Err(ModelError::NotSupported),
            chat_calls: Arc::new(Default::default()),
        }));
        registry.refresh_models().await;
        assert_eq!(registry.list_models().len(), 2);
        assert!(registry.model_info("emb-pro/e1").is_some());
    }

    #[tokio::test]
    async fn chat_routes_to_correct_provider() {
        let registry = ModelManager::new();
        let calls = Arc::new(std::sync::atomic::AtomicU32::new(0));
        registry.register_provider(Arc::new(StubProvider {
            id: "chat-pro",
            models: vec![model(
                "chat-pro/m1",
                ModelKind::Chat {
                    context_window: None,
                    max_output: None,
                    supports_stream: true,
                    capabilities: vec![],
                },
            )],
            chat_result: Ok(ModelChatResponse {
                model_id: String::new(),
                content: "ok".to_string(),
                finish_reason: Some("stop".to_string()),
                usage: Some(ModelTokenUsage {
                    input_tokens: 1,
                    output_tokens: 2,
                    total_tokens: 3,
                }),
                tool_calls: Vec::new(),
            }),
            embedding_result: Err(ModelError::NotSupported),
            chat_calls: calls.clone(),
        }));
        registry.refresh_models().await;
        let resp = registry
            .chat(ModelChatRequest {
                model_id: "chat-pro/m1".to_string(),
                messages: vec![ModelChatMessage::text(ModelChatRole::User, "hi")],
                temperature: None,
                max_tokens: None,
                reasoning_effort: None,
                tools: Vec::new(),
            })
            .await
            .unwrap();
        assert_eq!(resp.content, "ok");
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    /// 验证 chat 请求不会路由到 embedding 模型。
    #[tokio::test]
    async fn chat_rejects_embedding_model() {
        let registry = ModelManager::new();
        registry.register_provider(Arc::new(StubProvider {
            id: "embedding-pro",
            models: vec![model(
                "embedding-pro/m1",
                ModelKind::Embedding {
                    context_window: None,
                    dimensions: Some(3),
                    similarity: ModelSimilarity::Cosine,
                    capabilities: vec![],
                },
            )],
            chat_result: Ok(ModelChatResponse {
                model_id: String::new(),
                content: "must not be called".to_string(),
                finish_reason: None,
                usage: None,
                tool_calls: Vec::new(),
            }),
            embedding_result: Err(ModelError::NotSupported),
            chat_calls: Arc::new(Default::default()),
        }));
        registry.refresh_models().await;
        let err = registry
            .chat(ModelChatRequest {
                model_id: "embedding-pro/m1".to_string(),
                messages: vec![],
                temperature: None,
                max_tokens: None,
                reasoning_effort: None,
                tools: Vec::new(),
            })
            .await
            .unwrap_err();
        assert!(matches!(err, ModelError::NotSupported));
    }

    /// 验证 provider 返回额外向量时不会静默丢弃。
    #[tokio::test]
    async fn embedding_rejects_mismatched_vector_count() {
        let registry = ModelManager::new();
        registry.register_provider(Arc::new(StubProvider {
            id: "embedding-pro",
            models: vec![model(
                "embedding-pro/m1",
                ModelKind::Embedding {
                    context_window: None,
                    dimensions: Some(2),
                    similarity: ModelSimilarity::Cosine,
                    capabilities: vec![],
                },
            )],
            chat_result: Err(ModelError::NotSupported),
            embedding_result: Ok(ModelEmbeddingResponse {
                model_id: String::new(),
                dimensions: 2,
                vectors: vec![vec![1.0, 0.0], vec![0.0, 1.0]],
            }),
            chat_calls: Arc::new(Default::default()),
        }));
        registry.refresh_models().await;
        let err = registry
            .embedding(ModelEmbeddingRequest {
                model_id: "embedding-pro/m1".to_string(),
                input: vec!["text".to_string()],
                template_args: None,
                task_type: SemanticTask::RetrievalDocument,
                dimensions: None,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, ModelError::Transport(_)));
    }

    /// 验证 similarity 按模型元数据公式计算：cosine 对单位向量等于点积。
    #[tokio::test]
    async fn similarity_computes_cosine_for_embedding_model() {
        let registry = ModelManager::new();
        registry.register_provider(Arc::new(StubProvider {
            id: "embedding-pro",
            models: vec![model(
                "embedding-pro/m1",
                ModelKind::Embedding {
                    context_window: None,
                    dimensions: Some(3),
                    similarity: ModelSimilarity::Cosine,
                    capabilities: vec![],
                },
            )],
            chat_result: Err(ModelError::NotSupported),
            embedding_result: Err(ModelError::NotSupported),
            chat_calls: Arc::new(Default::default()),
        }));
        registry.refresh_models().await;
        let resp = registry
            .similarity(ModelSimilarityRequest {
                model_id: "embedding-pro/m1".to_string(),
                query: vec![1.0, 0.0, 0.0],
                targets: vec![
                    vec![1.0, 0.0, 0.0],
                    vec![0.0, 1.0, 0.0],
                    vec![0.0, 0.0, -1.0],
                ],
            })
            .await
            .unwrap();
        assert_eq!(resp.model_id, "embedding-pro/m1");
        assert_eq!(resp.similarities.len(), 3);
        assert!((resp.similarities[0] - 1.0).abs() < 1e-6);
        assert!((resp.similarities[1]).abs() < 1e-6);
        assert!((resp.similarities[2]).abs() < 1e-6);
    }

    /// 验证 similarity 拒绝 chat 模型与维度不一致向量。
    #[tokio::test]
    async fn similarity_rejects_non_embedding_and_mismatched_dims() {
        let registry = ModelManager::new();
        registry.register_provider(Arc::new(StubProvider {
            id: "chat-pro",
            models: vec![model(
                "chat-pro/m1",
                ModelKind::Chat {
                    context_window: None,
                    max_output: None,
                    supports_stream: false,
                    capabilities: vec![],
                },
            )],
            chat_result: Err(ModelError::NotSupported),
            embedding_result: Err(ModelError::NotSupported),
            chat_calls: Arc::new(Default::default()),
        }));
        registry.refresh_models().await;
        assert!(matches!(
            registry
                .similarity(ModelSimilarityRequest {
                    model_id: "chat-pro/m1".to_string(),
                    query: vec![1.0],
                    targets: vec![vec![1.0]],
                })
                .await,
            Err(ModelError::NotSupported)
        ));
        // embedding 模型但维度不一致
        registry.register_provider(Arc::new(StubProvider {
            id: "embedding-pro",
            models: vec![model(
                "embedding-pro/m1",
                ModelKind::Embedding {
                    context_window: None,
                    dimensions: Some(2),
                    similarity: ModelSimilarity::Cosine,
                    capabilities: vec![],
                },
            )],
            chat_result: Err(ModelError::NotSupported),
            embedding_result: Err(ModelError::NotSupported),
            chat_calls: Arc::new(Default::default()),
        }));
        registry.refresh_models().await;
        assert!(matches!(
            registry
                .similarity(ModelSimilarityRequest {
                    model_id: "embedding-pro/m1".to_string(),
                    query: vec![1.0, 0.0],
                    targets: vec![vec![1.0]],
                })
                .await,
            Err(ModelError::InvalidRequest(_))
        ));
    }

    /// 真余弦：非单位向量按模长归一化后计算（[3,4]·[4,3] = 24/25）。
    #[test]
    fn cosine_similarity_handles_non_unit_vectors() {
        let sim = cosine_similarity(&[3.0, 4.0], &[4.0, 3.0]);
        assert!((sim - 24.0 / 25.0).abs() < 1e-6);
        // 零向量视为不相似。
        assert_eq!(cosine_similarity(&[0.0, 0.0], &[1.0, 0.0]), 0.0);
        // 维度不一致视为不相似。
        assert_eq!(cosine_similarity(&[1.0], &[1.0, 0.0]), 0.0);
    }
}
