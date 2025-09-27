#[cfg(feature = "ai")]
use crate::core::ai::embedding_model::embedding_gemma::EmbeddingGemmaModel;
#[cfg(feature = "ai")]
use crate::core::ai::embedding_model::{EmbeddingModel, EmbeddingModelType};
#[cfg(feature = "ai")]
use crate::core::ai::model_manager::ModelManager;
use crate::error::AppResult;
use crate::program_manager::LaunchMethod;
use crate::program_manager::SemanticStoreItem;
/// 这个语义管理器用于：
/// 生成embedding向量
/// 管理程序的描述性信息
#[cfg(feature = "ai")]
use ndarray::Array1;
#[cfg(feature = "ai")]
use ndarray::ArrayView1;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
#[cfg_attr(not(feature = "ai"), allow(unused_imports))]
use tracing::debug;
pub trait GenerateEmbeddingForLoader {
    fn generate_embedding_for_loader(
        &self,
        show_name: &str,
        search_keywords: &str,
        launch_method: &LaunchMethod,
        description: &str,
    ) -> AppResult<EmbeddingVec>;
}

pub trait GenerateEmbeddingForManager {
    fn generate_embedding_for_manager(&self, user_input: &str) -> AppResult<EmbeddingVec>;
}

#[derive(Debug)]
pub struct SemanticManager {
    // 这个变量用于存储语义描述的信息(launch_method, semantic_item)
    semantic_store: Arc<RwLock<HashMap<String, SemanticStoreItem>>>,
    #[cfg(feature = "ai")]
    model_manager: Arc<ModelManager>,
}

impl SemanticManager {
    #[cfg(feature = "ai")]
    pub fn new(
        model_manager: Arc<ModelManager>,
        semantic_store: HashMap<String, SemanticStoreItem>,
    ) -> Self {
        let semantic_store = Arc::new(RwLock::new(semantic_store));

        Self {
            semantic_store,
            model_manager,
        }
    }

    #[cfg(not(feature = "ai"))]
    pub fn new(semantic_store: HashMap<String, SemanticStoreItem>) -> Self {
        let semantic_store = Arc::new(RwLock::new(semantic_store));

        Self { semantic_store }
    }

    pub fn get_semantic_descriptions(&self, launch_method: &LaunchMethod) -> String {
        let key = launch_method.get_text();
        self.semantic_store
            .read()
            .get(&key)
            .map(|item| item.description.clone())
            .unwrap_or_default()
    }

    pub fn update_semantic_store(&self, new_store: HashMap<String, SemanticStoreItem>) {
        let mut store = self.semantic_store.write();
        *store = new_store;
    }

    pub fn get_runtime_data(&self) -> HashMap<String, SemanticStoreItem> {
        let store = self.semantic_store.read();
        store.clone()
    }

    /// 基于模型类型计算相似度（更灵活的方案）
    #[cfg(feature = "ai")]
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
    #[cfg(feature = "ai")]
    pub fn compute_similarity(
        &self,
        embedding1: ArrayView1<f32>,
        embedding2: ArrayView1<f32>,
    ) -> f32 {
        // 目前只使用 EmbeddingGemma，但可以扩展为从配置中读取
        Self::compute_similarity_by_type(EmbeddingModelType::EmbeddingGemma, embedding1, embedding2)
    }
}

// 抽象一个内部别名，未启用 ai 时用 Vec<f32> 占位（更轻量且不依赖 ndarray）
#[cfg(feature = "ai")]
pub type EmbeddingVec = Array1<f32>;
#[cfg(not(feature = "ai"))]
pub type EmbeddingVec = Vec<f32>;

impl GenerateEmbeddingForLoader for SemanticManager {
    #[cfg_attr(not(feature = "ai"), allow(unused_variables))]
    fn generate_embedding_for_loader(
        &self,
        show_name: &str,
        search_keywords: &str,
        launch_method: &LaunchMethod,
        description: &str,
    ) -> AppResult<EmbeddingVec> {
        #[cfg(feature = "ai")]
        {
            // 目前只会使用 EmbeddingGemma 模型作为语义搜索器，所以这里就直接硬编码了，后面如果会添加新的模型，再做对应的解耦处理
            let embedding_model =
                self.model_manager.load_embedding_model(EmbeddingModelType::EmbeddingGemma)?;

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
        #[cfg(not(feature = "ai"))]
        {
            // 未启用 ai，返回空向量
            Ok(Vec::new())
        }
    }
}

impl GenerateEmbeddingForManager for SemanticManager {
    #[cfg_attr(not(feature = "ai"), allow(unused_variables))]
    fn generate_embedding_for_manager(&self, user_input: &str) -> AppResult<EmbeddingVec> {
        #[cfg(feature = "ai")]
        {
            // 目前只会使用 EmbeddingGemma 模型作为语义搜索器，所以这里就直接硬编码了，后面如果会添加新的模型，再做对应的解耦处理
            let embedding_model =
                self.model_manager.load_embedding_model(EmbeddingModelType::EmbeddingGemma)?;

            let query = format!("task: search result | query: {}", user_input);
            debug!("用户输入: {}", query);

            let mut embedding_model_lock = embedding_model.lock();
            let result = embedding_model_lock.compute_embedding(&query)?;
            Ok(result)
        }
        #[cfg(not(feature = "ai"))]
        {
            Ok(Vec::new())
        }
    }
}
