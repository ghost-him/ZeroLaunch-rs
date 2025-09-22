pub mod embedding_gemma;
pub mod paraphrase_multilingual_miniml;
use std::fmt::Debug;

use crate::core::ai::{
    embedding_model::{
        embedding_gemma::EmbeddingGemmaModel,
        paraphrase_multilingual_miniml::ParaphraseMultilingualMiniLM,
    },
    OnnxModelConfig,
};
use crate::Arc;
use ndarray::{Array1, ArrayView1};
use parking_lot::Mutex;
/// Embedding模型trait定义
pub trait EmbeddingModel: Debug + Send + Sync {
    /// 初始化模型
    fn new(config: OnnxModelConfig) -> ort::Result<Self>
    where
        Self: Sized;

    /// 计算文本的embedding向量
    fn compute_embedding(&mut self, text: &str) -> ort::Result<Array1<f32>>;

    /// 批量计算文本的embedding向量
    fn compute_embeddings(&mut self, texts: &[&str]) -> ort::Result<Vec<Array1<f32>>>;

    /// 计算两个embedding向量的相似度（返回百分比）
    fn compute_similarity(embedding1: ArrayView1<f32>, embedding2: ArrayView1<f32>) -> f32
    where
        Self: Sized;

    /// 更新后端配置
    fn update_backend(&mut self, config: OnnxModelConfig) -> ort::Result<()>;

    /// 获取当前配置
    fn get_runtime_config(&self) -> &OnnxModelConfig;

    /// 获取模型的固定配置信息
    fn get_default_config() -> OnnxModelConfig
    where
        Self: Sized;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EmbeddingModelType {
    EmbeddingGemma,
    //ParaphraseMultilingualMiniLM,
}

impl EmbeddingModelType {
    fn get_config(&self) -> OnnxModelConfig {
        match self {
            EmbeddingModelType::EmbeddingGemma => EmbeddingGemmaModel::get_default_config(),
            //EmbeddingModelType::ParaphraseMultilingualMiniLM => ParaphraseMultilingualMiniLM::get_default_config(),
        }
    }

    pub fn generate_model(&self) -> ort::Result<Arc<Mutex<dyn EmbeddingModel>>> {
        match self {
            EmbeddingModelType::EmbeddingGemma => Ok(Arc::new(Mutex::new(
                EmbeddingGemmaModel::new(self.get_config())?,
            ))),
            //EmbeddingModelType::ParaphraseMultilingualMiniLM => Ok(Arc::new(Mutex::new(ParaphraseMultilingualMiniLM::new(self.get_config())?))),
        }
    }
}
