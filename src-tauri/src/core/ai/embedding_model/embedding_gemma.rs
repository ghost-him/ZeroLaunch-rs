use crate::commands::debug;
use crate::core::ai::ai_loader::setup_session_and_tokenizer;
use crate::core::ai::embedding_model::{Array1, ArrayView1};
use crate::core::ai::{embedding_model::EmbeddingModel, OnnxModelConfig};
use ndarray::Axis;
use ort::session::Session;
use ort::value::TensorRef;
use ort::Error;
use rayon::prelude::*;
use serde_json::Value;
use std::fmt;
use tokenizers::Tokenizer;

pub struct EmbeddingGemmaModel {
    session: Session,
    tokenizer: Tokenizer,
    config: OnnxModelConfig,
}

impl fmt::Debug for EmbeddingGemmaModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EmbeddingGemmaModel")
    }
}

impl EmbeddingModel for EmbeddingGemmaModel {
    fn new(config: OnnxModelConfig) -> ort::Result<Self> {
        let (session, mut tokenizer) = setup_session_and_tokenizer(&config)?;

        // Load tokenizer config
        let config_content = std::fs::read_to_string(&config.tokenizer_config_path)
            .map_err(|e| Error::new(format!("Failed to read tokenizer config: {}", e)))?;
        let tokenizer_config: Value = serde_json::from_str(&config_content)
            .map_err(|e| Error::new(format!("Failed to parse tokenizer config: {}", e)))?;

        // Configure padding
        if let Some(pad_token) = tokenizer_config["pad_token"].as_str() {
            tokenizer.with_padding(Some(tokenizers::PaddingParams {
                strategy: tokenizers::PaddingStrategy::BatchLongest,
                direction: tokenizers::PaddingDirection::Right,
                pad_to_multiple_of: None,
                pad_id: 0,
                pad_type_id: 0,
                pad_token: pad_token.to_string(),
            }));
        }

        Ok(Self {
            session,
            tokenizer,
            config,
        })
    }

    fn compute_embedding(&mut self, text: &str) -> ort::Result<Array1<f32>> {
        let embeddings = self.compute_embeddings(&[text])?;
        Ok(embeddings.into_iter().next().unwrap_or_default())
    }

    fn compute_embeddings(&mut self, texts: &[&str]) -> ort::Result<Vec<Array1<f32>>> {
        // Tokenize inputs
        let encodings = self
            .tokenizer
            .encode_batch(texts.to_vec(), true)
            .map_err(|e| Error::new(e.to_string()))?;

        let padded_token_length = encodings[0].len();

        // Prepare input tensors
        let total_tokens = texts.len() * padded_token_length;
        let mut ids = Vec::with_capacity(total_tokens);
        let mut mask = Vec::with_capacity(total_tokens);

        let ids_temp: Vec<i64> = encodings
            .par_iter()
            .flat_map(|e| e.get_ids().par_iter().map(|i| *i as i64))
            .collect();
        let mask_temp: Vec<i64> = encodings
            .par_iter()
            .flat_map(|e| e.get_attention_mask().par_iter().map(|i| *i as i64))
            .collect();
        ids.extend(ids_temp);
        mask.extend(mask_temp);

        // Create tensor references
        let a_ids = TensorRef::from_array_view(([texts.len(), padded_token_length], &*ids))?;
        let a_mask = TensorRef::from_array_view(([texts.len(), padded_token_length], &*mask))?;

        // Run inference
        let outputs = self.session.run(ort::inputs![a_ids, a_mask])?;
        let embeddings_raw = outputs[0].try_extract_array::<f32>()?.to_owned();

        // Perform mean pooling
        let mut embeddings = embeddings_raw.mean_axis(Axis(1)).unwrap();

        // L2 normalization

        embeddings.axis_iter_mut(Axis(0)).for_each(|mut row| {
            let norm = row.iter().map(|x| x * x).sum::<f32>().sqrt();
            if norm > 0.0 {
                row.mapv_inplace(|x| x / norm);
            }
        });

        // Convert to Vec<Array1<f32>>
        let result: Vec<Array1<f32>> = embeddings
            .axis_iter(Axis(0))
            .map(|row| row.to_owned().into_dimensionality().unwrap())
            .collect();

        Ok(result)
    }

    fn compute_similarity(embedding1: ArrayView1<f32>, embedding2: ArrayView1<f32>) -> f32 {
        if embedding1.len() != embedding2.len() {
            return 0.0;
        }

        // 使用ndarray的高效向量运算
        let dot_product = embedding1.dot(&embedding2);
        let norm1 = embedding1.dot(&embedding1).sqrt();
        let norm2 = embedding2.dot(&embedding2).sqrt();

        if norm1 == 0.0 || norm2 == 0.0 {
            return 0.0;
        }

        // 返回百分比形式的余弦相似度
        (dot_product / (norm1 * norm2)) * 100.0
    }

    fn update_backend(&mut self, new_config: OnnxModelConfig) -> ort::Result<()> {
        // Create new model instance with updated config
        let new_model = Self::new(new_config)?;

        // Replace current instance
        self.session = new_model.session;
        self.tokenizer = new_model.tokenizer;
        self.config = new_model.config;

        Ok(())
    }

    fn get_runtime_config(&self) -> &OnnxModelConfig {
        &self.config
    }

    fn get_default_config() -> OnnxModelConfig {
        OnnxModelConfig {
            model_path: "EmbeddingGemma-300m/model.onnx".to_string(),
            tokenizer_path: "EmbeddingGemma-300m/tokenizer.json".to_string(),
            tokenizer_config_path: "EmbeddingGemma-300m/tokenizer_config.json".to_string(),
            ..Default::default()
        }
    }
}
