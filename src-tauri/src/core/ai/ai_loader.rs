// ai模型加载器
use crate::core::ai::embedding_model::EmbeddingModel;
use crate::core::ai::embedding_model::EmbeddingModelType;
use crate::core::ai::text_generation_model::TextGenerationModelType;
use crate::core::ai::GraphOptimizationLevel;
use crate::core::ai::OnnxModelConfig;
use crate::Arc;
use once_cell::sync::OnceCell;
use ort::session::Session;
use ort::Error;
use parking_lot::Mutex;
use tokenizers::Tokenizer;
use tracing::info;
use tracing::warn;
#[derive(Debug)]
pub struct AILoader {}

impl AILoader {
    pub fn new() -> Self {
        let once_cell = OnceCell::new();
        let result = once_cell.get_or_try_init(|| {
            info!("Initializing ORT...");
            ort::init().commit()
        });
        if let Err(e) = result {
            warn!("Failed to initialize ORT: {:?}", e);
        }
        Self {}
    }

    // pub fn load_text_generation_model(&self, model_type: TextGenerationModelType) -> ort::Result<TextGenerationModel> {
    //     match model_type {
    //         TextGenerationModelType::DeepSeekR1Distill => {

    //         }
    //     }
    // }

    pub fn load_embedding_model(
        &self,
        model_type: EmbeddingModelType,
    ) -> ort::Result<Arc<Mutex<dyn EmbeddingModel>>> {
        model_type.generate_model()
    }
}

pub fn setup_session_and_tokenizer(config: &OnnxModelConfig) -> ort::Result<(Session, Tokenizer)> {
    // 步骤1: 创建并配置 Session Builder
    let session_builder = Session::builder()?
        .with_optimization_level(GraphOptimizationLevel::Level3)?
        .with_execution_providers([
            ort::execution_providers::CUDAExecutionProvider::default().build(),
            ort::execution_providers::DirectMLExecutionProvider::default().build(),
            ort::execution_providers::XNNPACKExecutionProvider::default().build(),
            ort::execution_providers::CPUExecutionProvider::default().build(),
        ])?;

    // 步骤 2: 加载模型
    let session = session_builder.commit_from_file(&config.model_path)?;

    // 步骤 3: 加载 Tokenizer
    let tokenizer = Tokenizer::from_file(&config.tokenizer_path)
        .map_err(|e| Error::new(format!("Failed to load tokenizer: {}", e)))?;

    Ok((session, tokenizer))
}
