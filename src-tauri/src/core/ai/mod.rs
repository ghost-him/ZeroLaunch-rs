use ort::session::builder::GraphOptimizationLevel;
pub mod ai_loader;
pub mod embedding_model;
pub mod model_manager;
pub mod text_generation_model;

/// Onnx模型配置结构体
#[derive(Debug)]
pub struct OnnxModelConfig {
    /// 是否启用DirectML后端
    pub enable_directml: bool,
    /// 是否启用cuda后端
    pub enable_cuda: bool,
    /// 模型文件路径
    pub model_path: String,
    /// Tokenizer文件路径
    pub tokenizer_path: String,
    /// Tokenizer配置文件路径
    pub tokenizer_config_path: String,
}

impl Default for OnnxModelConfig {
    fn default() -> Self {
        Self {
            enable_directml: true,
            enable_cuda: true,
            model_path: String::new(),
            tokenizer_path: String::new(),
            tokenizer_config_path: String::new(),
        }
    }
}
