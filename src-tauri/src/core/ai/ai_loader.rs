// ai模型加载器
use crate::core::ai::embedding_model::EmbeddingModel;
use crate::core::ai::embedding_model::EmbeddingModelType;
use crate::core::ai::GraphOptimizationLevel;
use crate::core::ai::OnnxModelConfig;
use crate::error::OptionExt;
use crate::Arc;
use once_cell::sync::OnceCell;
use ort::session::Session;
use ort::Error;
use parking_lot::Mutex;
use ort::execution_providers::CPUExecutionProvider;
use ort::execution_providers::XNNPACKExecutionProvider;
use tokenizers::Tokenizer;
use tracing::info;
use tracing::warn;
#[derive(Debug)]
pub struct AILoader {}

impl Default for AILoader {
    fn default() -> Self {
        Self::new()
    }
}

static ORT_INIT: OnceCell<()> = OnceCell::new();

impl AILoader {
    pub fn new() -> Self {
        // 全局只初始化一次 ORT
        let _ = ORT_INIT.get_or_init(|| {
            if let Err(e) = ort::init().commit() {
                warn!("Failed to initialize ORT: {:?}", e);
            } else {
                info!("ORT initialized");
            }
        });
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
    let mut session: Option<Session> = None;
    // Attempt XNNPACK + CPU
    if session.is_none() {
        match Session::builder()
            .and_then(|b| b.with_optimization_level(GraphOptimizationLevel::Level3))
            .and_then(|b| {
                b.with_execution_providers([
                    XNNPACKExecutionProvider::default().build(),
                    CPUExecutionProvider::default().build(),
                ])
            })
            .and_then(|b| b.commit_from_file(&config.model_path))
        {
            Ok(s) => {
                info!("Using execution providers: XNNPACK, CPU");
                session = Some(s);
            }
            Err(e) => warn!("Failed to init XNNPACK, CPU providers: {:?}", e),
        }
    }

    // CPU Only
    if session.is_none() {
        info!("Falling back to CPU execution provider only");
        session = Some(
            Session::builder()?
                .with_optimization_level(GraphOptimizationLevel::Level3)?
                .with_execution_providers([CPUExecutionProvider::default().build()])?
                .commit_from_file(&config.model_path)?,
        );
    }

    let session = session.expect_programming("CPU only session creation should never fail if we reached here");
    let tokenizer = Tokenizer::from_file(&config.tokenizer_path)
        .map_err(|e| Error::new(format!("Failed to load tokenizer: {}", e)))?;

    Ok((session, tokenizer))
}