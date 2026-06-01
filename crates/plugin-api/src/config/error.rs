/// 配置错误类型
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Component not found: {0}")]
    NotFound(String),

    #[error("Configuration validation failed: {0}")]
    ValidationFailed(String),

    #[error("Invalid setting value for key '{key}': {message}")]
    InvalidValue { key: String, message: String },

    #[error("Configuration apply failed: {0}")]
    ApplyFailed(String),

    #[error("Persistence error: {0}")]
    PersistenceError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}
