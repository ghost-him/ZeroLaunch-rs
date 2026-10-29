/// 配置错误类型
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Component not found: {0}")]
    NotFound(String),

    /// 校验失败的人读消息：由产生方给出完整句子，不再套英文前缀，
    /// 避免中文消息被包装成「Configuration validation failed: 中文」的混排。
    #[error("{0}")]
    ValidationFailed(String),

    #[error("Invalid setting value for key '{key}': {message}")]
    InvalidValue { key: String, message: String },

    /// 应用或动作失败的人读消息：同 ValidationFailed，由产生方给出完整句子。
    #[error("{0}")]
    ApplyFailed(String),

    #[error("Persistence error: {0}")]
    PersistenceError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}
