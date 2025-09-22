use crate::core::ai::embedding_model::EmbeddingModel;
use crate::core::ai::OnnxModelConfig;
use ort::session::Session;
use tokenizers::{Tokenizer, TokenizerBuilder};

pub struct ParaphraseMultilingualMiniLM {
    session: Session,
    tokenizer: Tokenizer,
    config: OnnxModelConfig,
}

// impl EmbeddingModel for ParaphraseMultilingualMiniLM {
//     fn new(config: OnnxModelConfig) -> ort::Result<Self> where Self: Sized {
//         let session = Session::builder()?
//             .with_optimization_level(OptimizationLevel::Basic)?
//             .with_model_from_file(config.model_path)?;
//         let tokenizer = Tokenizer::from_file(config.tokenizer_path)?;
//         Ok(Self { session, tokenizer, config })
//     }

//     fn get_default_config() -> OnnxModelConfig {
//         OnnxModelConfig {
//             model_path: "paraphrase_multilingual_minilm.onnx".to_string(),
//             tokenizer_path: "paraphrase_multilingual_minilm.tokenizer.json".to_string(),
//             tokenizer_config_path: "paraphrase_multilingual_minilm.tokenizer_config.json".to_string(),
//             ..Default::default()
//         }
//     }
// }
