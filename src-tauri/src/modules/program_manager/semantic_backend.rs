use std::sync::Arc;

use super::semantic_manager::EmbeddingBackend;
#[cfg(feature = "ai")]
use tracing::debug;

#[cfg(feature = "ai")]
use super::unit::{EmbeddingVec, LaunchMethod};
#[cfg(feature = "ai")]
use crate::core::ai::embedding_model::embedding_gemma::EmbeddingGemmaModel;
#[cfg(feature = "ai")]
use crate::core::ai::embedding_model::{EmbeddingModel, EmbeddingModelType};
#[cfg(feature = "ai")]
use crate::core::ai::model_manager::ModelManager;
#[cfg(feature = "ai")]
use ndarray::ArrayView1;
#[cfg(feature = "ai")]
use std::path::Path;

/// 根据当前特性构建语义后端。
#[cfg(feature = "ai")]
pub fn create_embedding_backend(
    model_manager: Arc<ModelManager>,
) -> Option<Arc<dyn EmbeddingBackend>> {
    debug!("模型管理器由外部提供，开始构建语义后端");
    let backend: Arc<dyn EmbeddingBackend> = Arc::new(AiEmbeddingBackend::new(model_manager));
    Some(backend)
}

/// 根据当前特性构建语义后端（无 AI 场景返回 None）。
#[cfg(not(feature = "ai"))]
pub fn create_embedding_backend() -> Option<Arc<dyn EmbeddingBackend>> {
    None
}

#[cfg(feature = "ai")]
struct AiEmbeddingBackend {
    model_manager: Arc<ModelManager>,
}

#[cfg(feature = "ai")]
impl AiEmbeddingBackend {
    fn new(model_manager: Arc<ModelManager>) -> Self {
        Self { model_manager }
    }
}

#[cfg(feature = "ai")]
impl EmbeddingBackend for AiEmbeddingBackend {
    fn generate_embedding_for_loader(
        &self,
        show_name: &str,
        search_keywords: &str,
        launch_method: &LaunchMethod,
        description: &str,
    ) -> crate::error::AppResult<EmbeddingVec> {
        let embedding_model = self
            .model_manager
            .load_embedding_model(EmbeddingModelType::EmbeddingGemma)?;

        let context = format!(
            "软件名字:{}，也叫做:{}，启动地址或uwp包族名:{}，描述信息:{}",
            show_name,
            search_keywords,
            launch_method.get_text(),
            description
        );
        let combined_text = format!("title: {} | text: {}", show_name, context);

        let mut embedding_model_lock = embedding_model.lock();
        let result = embedding_model_lock.compute_embedding(&combined_text)?;
        Ok(result.to_vec())
    }

    fn generate_embedding_for_manager(
        &self,
        user_input: &str,
    ) -> crate::error::AppResult<EmbeddingVec> {
        let embedding_model = self
            .model_manager
            .load_embedding_model(EmbeddingModelType::EmbeddingGemma)?;

        let query = format!("task: search result | query: {}", user_input);
        let mut embedding_model_lock = embedding_model.lock();
        let result = embedding_model_lock.compute_embedding(&query)?;
        Ok(result.to_vec())
    }

    fn compute_similarity(&self, embedding1: &EmbeddingVec, embedding2: &EmbeddingVec) -> f32 {
        let view1 = ArrayView1::from(&embedding1[..]);
        let view2 = ArrayView1::from(&embedding2[..]);
        EmbeddingGemmaModel::compute_similarity(view1, view2)
    }

    fn release_resources(&self) {
        debug!("Releasing cached embedding model");
        self.model_manager
            .release_embedding_model(EmbeddingModelType::EmbeddingGemma);
    }

    fn is_ready(&self) -> bool {
        // 粗略检测：检查默认模型文件是否存在
        let cfg = EmbeddingModelType::EmbeddingGemma.get_config();
        Path::new(&cfg.model_path).exists()
            && Path::new(&cfg.tokenizer_path).exists()
            && Path::new(&cfg.tokenizer_config_path).exists()
    }
}
