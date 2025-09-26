// 这个模块用于管理模型的生命周期，包括加载、卸载、缓存等操作。
// 用户如果想要调用模型，则都是通过这个管理器调用的，同时只需要传入要调用什么模型，而不用管具体的加载细节与模型的类型，也不用关心模型的配置细节
use crate::core::ai::ai_loader::AILoader;
use crate::core::ai::embedding_model::EmbeddingModel;
use crate::core::ai::embedding_model::EmbeddingModelType;
use crate::Arc;
use dashmap::DashMap;
use parking_lot::Mutex;
#[derive(Debug)]
pub struct ModelManager {
    ai_loader: AILoader,
    //text_generation_models: HashMap<TextGenerationModelType, Arc<Mutex<TextGenerationModel>>>,
    embedding_models: DashMap<EmbeddingModelType, Arc<Mutex<dyn EmbeddingModel>>>,
}

impl Default for ModelManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelManager {
    pub fn new() -> Self {
        let ai_loader = AILoader::new();
        Self {
            ai_loader,
            //text_generation_models: HashMap::new(),
            embedding_models: DashMap::new(),
        }
    }

    // pub fn load_text_generation_model(&self, model_type: TextGenerationModelType) -> ort::Result<TextGenerationModel> {
    //     self.ai_loader.load_text_generation_model(model_type)
    // }

    pub fn load_embedding_model(
        &self,
        model_type: EmbeddingModelType,
    ) -> ort::Result<Arc<Mutex<dyn EmbeddingModel>>> {
        use dashmap::mapref::entry::Entry;
        match self.embedding_models.entry(model_type) {
            Entry::Occupied(e) => Ok(e.get().clone()),
            Entry::Vacant(v) => {
                let model = self.ai_loader.load_embedding_model(model_type)?;
                let cloned = model.clone();
                v.insert(cloned.clone());
                Ok(cloned)
            }
        }
    }
}
