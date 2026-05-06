use serde::Serialize;
use std::fmt;

/// 前后端通信统一错误类型。
/// 用于所有 Tauri command 的 Err 变体，前端可据此展示用户友好的错误提示。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeError {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    InvalidQuery,
    ComponentNotFound,
    ValidationFailed,
    ActionFailed,
    PluginError,
    ConfigError,
    NetworkError,
    InternalError,
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorCode::InvalidQuery => write!(f, "INVALID_QUERY"),
            ErrorCode::ComponentNotFound => write!(f, "COMPONENT_NOT_FOUND"),
            ErrorCode::ValidationFailed => write!(f, "VALIDATION_FAILED"),
            ErrorCode::ActionFailed => write!(f, "ACTION_FAILED"),
            ErrorCode::PluginError => write!(f, "PLUGIN_ERROR"),
            ErrorCode::ConfigError => write!(f, "CONFIG_ERROR"),
            ErrorCode::NetworkError => write!(f, "NETWORK_ERROR"),
            ErrorCode::InternalError => write!(f, "INTERNAL_ERROR"),
        }
    }
}

impl fmt::Display for BridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for BridgeError {}

impl From<crate::core::types::ConfigError> for BridgeError {
    fn from(e: crate::core::types::ConfigError) -> Self {
        let code = match &e {
            crate::core::types::ConfigError::NotFound(_) => ErrorCode::ComponentNotFound,
            crate::core::types::ConfigError::ValidationFailed(_) => ErrorCode::ValidationFailed,
            _ => ErrorCode::ConfigError,
        };
        BridgeError {
            code,
            message: e.to_string(),
            details: None,
            component_id: None,
        }
    }
}

impl From<crate::plugin_system::types::PluginError> for BridgeError {
    fn from(e: crate::plugin_system::types::PluginError) -> Self {
        BridgeError {
            code: ErrorCode::PluginError,
            message: e.to_string(),
            details: None,
            component_id: None,
        }
    }
}

impl BridgeError {
    pub fn not_found(component_id: &str) -> Self {
        BridgeError {
            code: ErrorCode::ComponentNotFound,
            message: format!("Component not found: {}", component_id),
            details: None,
            component_id: Some(component_id.to_string()),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        BridgeError {
            code: ErrorCode::InternalError,
            message: message.into(),
            details: None,
            component_id: None,
        }
    }

    pub fn validation_failed(message: impl Into<String>) -> Self {
        BridgeError {
            code: ErrorCode::ValidationFailed,
            message: message.into(),
            details: None,
            component_id: None,
        }
    }
}
