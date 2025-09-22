use crate::core::ai::embedding_model::embedding_gemma::EmbeddingGemmaModel;
use crate::core::ai::embedding_model::{EmbeddingModel, EmbeddingModelType};
use crate::core::ai::model_manager::ModelManager;
use crate::error::AppResult;
use crate::program_manager::LaunchMethod;
/// 这个语义管理器用于：
/// 生成embedding向量
/// 管理程序的描述性信息
use ndarray::Array1;
use ndarray::ArrayView1;
use std::sync::Arc;
use tracing::debug;
pub trait GenerateEmbeddingForLoader {
    fn generate_embedding_for_loader(
        &self,
        show_name: &str,
        search_keywords: &str,
        launch_method: &LaunchMethod,
        description: &str,
    ) -> AppResult<Array1<f32>>;
}

pub trait GenerateEmbeddingForManager {
    fn generate_embedding_for_manager(&self, user_input: &str) -> AppResult<Array1<f32>>;
}

#[derive(Debug)]
pub struct SemanticManager {
    model_manager: Option<Arc<ModelManager>>,
}

impl SemanticManager {
    pub fn new(model_manager: Option<Arc<ModelManager>>) -> Self {
        Self { model_manager }
    }

    /// 基于模型类型计算相似度（更灵活的方案）
    pub fn compute_similarity_by_type(
        model_type: EmbeddingModelType,
        embedding1: ArrayView1<f32>,
        embedding2: ArrayView1<f32>,
    ) -> f32 {
        match model_type {
            EmbeddingModelType::EmbeddingGemma => {
                EmbeddingGemmaModel::compute_similarity(embedding1, embedding2)
            }
        }
    }

    /// 使用当前配置的模型计算相似度
    pub fn compute_similarity(
        &self,
        embedding1: ArrayView1<f32>,
        embedding2: ArrayView1<f32>,
    ) -> f32 {
        // 目前只使用 EmbeddingGemma，但可以扩展为从配置中读取
        Self::compute_similarity_by_type(EmbeddingModelType::EmbeddingGemma, embedding1, embedding2)
    }
}

impl GenerateEmbeddingForLoader for SemanticManager {
    fn generate_embedding_for_loader(
        &self,
        show_name: &str,
        search_keywords: &str,
        launch_method: &LaunchMethod,
        description: &str,
    ) -> AppResult<Array1<f32>> {
        let Some(ref model_manager) = self.model_manager else {
            // 如果没有模型管理器，返回空的embedding
            return Ok(Array1::zeros(0));
        };

        // 目前只会使用 EmbeddingGemma 模型作为语义搜索器，所以这里就直接硬编码了，后面如果会添加新的模型，再做对应的解耦处理
        let embedding_model =
            model_manager.load_embedding_model(EmbeddingModelType::EmbeddingGemma)?;

        let title = show_name;
        let context = format!(
            "软件名字:{}，也叫做:{}，启动地址或uwp包族名:{}，描述信息:{}",
            show_name,
            search_keywords,
            launch_method.get_text(),
            description
        );
        let combined_text = format!("title: {} | text: {}", title, context);
        debug!("生成embedding的文本: {}", combined_text);

        let mut embedding_model_lock = embedding_model.lock();
        let result = embedding_model_lock.compute_embedding(&combined_text)?;
        Ok(result)
    }
}

impl GenerateEmbeddingForManager for SemanticManager {
    fn generate_embedding_for_manager(&self, user_input: &str) -> AppResult<Array1<f32>> {
        let Some(ref model_manager) = self.model_manager else {
            // 如果没有模型管理器，返回空的embedding
            return Ok(Array1::zeros(0));
        };

        // 目前只会使用 EmbeddingGemma 模型作为语义搜索器，所以这里就直接硬编码了，后面如果会添加新的模型，再做对应的解耦处理
        let embedding_model =
            model_manager.load_embedding_model(EmbeddingModelType::EmbeddingGemma)?;

        let query = format!("task: search result | query: {}", user_input);
        debug!("用户输入: {}", query);

        let mut embedding_model_lock = embedding_model.lock();
        let result = embedding_model_lock.compute_embedding(&query)?;
        Ok(result)
    }
}
