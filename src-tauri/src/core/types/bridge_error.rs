use serde::Serialize;
use std::fmt;
use zerolaunch_plugin_api::HostApiError;
/// 前后端通信统一错误类型。
/// 用于所有 Tauri command 的 Err 变体，前端可据此展示用户友好的错误提示。
#[derive(Debug, Clone, Serialize)]
pub struct BridgeError {
    #[serde(rename = "code")]
    pub code: ErrorCode,
    #[serde(rename = "message")]
    pub message: String,
    #[serde(rename = "details")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    #[serde(rename = "componentId")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub component_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub enum ErrorCode {
    #[serde(rename = "INVALID_QUERY")]
    InvalidQuery,
    #[serde(rename = "COMPONENT_NOT_FOUND")]
    ComponentNotFound,
    #[serde(rename = "VALIDATION_FAILED")]
    ValidationFailed,
    #[serde(rename = "ACTION_FAILED")]
    ActionFailed,
    #[serde(rename = "PLUGIN_ERROR")]
    PluginError,
    #[serde(rename = "CONFIG_ERROR")]
    ConfigError,
    #[serde(rename = "NETWORK_ERROR")]
    NetworkError,
    #[serde(rename = "INTERNAL_ERROR")]
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

impl From<HostApiError> for BridgeError {
    fn from(e: HostApiError) -> Self {
        match &e {
            HostApiError::PathTraversalRejected { .. } => BridgeError {
                code: ErrorCode::ValidationFailed,
                message: e.to_string(),
                details: None,
                component_id: None,
            },
            _ => BridgeError {
                code: ErrorCode::InternalError,
                message: e.to_string(),
                details: None,
                component_id: None,
            },
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
