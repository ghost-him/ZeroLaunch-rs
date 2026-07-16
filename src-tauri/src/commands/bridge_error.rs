use serde::Serialize;
use std::fmt;
use zerolaunch_plugin_api::config::ConfigError;
use zerolaunch_plugin_api::HostApiError;

use crate::plugin_framework::PluginManagerError;
use crate::plugin_framework::SessionRouterError;

/// 前后端通信统一错误类型。这个只可以用于前端后通信，不可以在内部模块使用该错误。内部模块的错误应该自己定义自己的错误。
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
    #[serde(rename = "traceId", serialize_with = "serialize_trace_id")]
    pub trace_id: String,
}

/// 序列化 trace_id，空字符串触发 warning 日志以暴露遗漏（debug 模式额外 panic）。
fn serialize_trace_id<S>(value: &str, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    if value.is_empty() {
        debug_assert!(
            false,
            "BridgeError 缺少 trace_id，说明错误路径未调用 with_trace_id"
        );
        tracing::warn!("BridgeError 缺少 trace_id，某错误路径未调用 with_trace_id");
    }
    serializer.serialize_str(value)
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

impl From<ConfigError> for BridgeError {
    fn from(e: ConfigError) -> Self {
        let code = match &e {
            ConfigError::NotFound(_) => ErrorCode::ComponentNotFound,
            ConfigError::ValidationFailed(_) => ErrorCode::ValidationFailed,
            _ => ErrorCode::ConfigError,
        };
        BridgeError {
            code,
            message: e.to_string(),
            details: None,
            component_id: None,
            trace_id: String::new(),
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
                trace_id: String::new(),
            },
            _ => BridgeError {
                code: ErrorCode::InternalError,
                message: e.to_string(),
                details: None,
                component_id: None,
                trace_id: String::new(),
            },
        }
    }
}

impl From<zerolaunch_plugin_api::PluginError> for BridgeError {
    fn from(e: zerolaunch_plugin_api::PluginError) -> Self {
        BridgeError {
            code: ErrorCode::PluginError,
            message: e.to_string(),
            details: None,
            component_id: None,
            trace_id: String::new(),
        }
    }
}

/// 将内部 SessionRouterError 转换为 IPC 边界使用的 BridgeError。
impl From<SessionRouterError> for BridgeError {
    fn from(e: SessionRouterError) -> Self {
        match e {
            SessionRouterError::NotInitialized(msg) | SessionRouterError::InvalidState(msg) => {
                BridgeError::internal(msg)
            }
            SessionRouterError::CandidateNotFound(id) => {
                BridgeError::not_found(&format!("Candidate {}", id))
            }
            SessionRouterError::InvalidPayload(msg) => BridgeError::validation_failed(msg),
            SessionRouterError::PluginError(msg)
            | SessionRouterError::ExecutionError(msg)
            | SessionRouterError::Internal(msg) => BridgeError::internal(msg),
        }
    }
}

/// 将内部 PluginManagerError 转换为 IPC 边界使用的 BridgeError。
impl From<PluginManagerError> for BridgeError {
    fn from(e: PluginManagerError) -> Self {
        match e {
            PluginManagerError::PluginNotFound(msg) | PluginManagerError::FileNotFound(msg) => {
                BridgeError::not_found(&msg)
            }
            PluginManagerError::UnsupportedFormat(msg) => BridgeError::validation_failed(msg),
            PluginManagerError::Internal(msg) => BridgeError::internal(msg),
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
            trace_id: String::new(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        BridgeError {
            code: ErrorCode::InternalError,
            message: message.into(),
            details: None,
            component_id: None,
            trace_id: String::new(),
        }
    }

    pub fn validation_failed(message: impl Into<String>) -> Self {
        BridgeError {
            code: ErrorCode::ValidationFailed,
            message: message.into(),
            details: None,
            component_id: None,
            trace_id: String::new(),
        }
    }

    /// 注入 trace_id 到当前 BridgeError。
    pub fn with_trace_id(mut self, trace_id: &str) -> Self {
        self.trace_id = trace_id.to_string();
        self
    }
}

/// 为 Result<T, E: Into<BridgeError>> 提供链式 trace_id 注入。
/// 用法: `some_result.with_trace_id(&trace_id)?`
pub trait WithTraceId<T> {
    fn with_trace_id(self, trace_id: &str) -> Result<T, BridgeError>;
}

impl<T, E: Into<BridgeError>> WithTraceId<T> for Result<T, E> {
    fn with_trace_id(self, trace_id: &str) -> Result<T, BridgeError> {
        self.map_err(|e| e.into().with_trace_id(trace_id))
    }
}
