use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("serde: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("toml: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("semver: {0}")]
    Semver(#[from] semver::Error),

    #[error("invalid frame: {0}")]
    InvalidFrame(String),

    #[error("rpc error: code={code} message={message}")]
    Rpc { code: i32, message: String },

    #[error("timeout")]
    Timeout,

    #[error("transport closed")]
    TransportClosed,

    #[error("manifest error: {0}")]
    Manifest(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl JsonRpcError {
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }
}

/// JSON-RPC 2.0 standard and custom error codes.
pub mod codes {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;
    pub const PLUGIN_ERROR: i32 = -32000;
    pub const PLUGIN_CRASHED: i32 = -32001;
    pub const TIMEOUT_ERROR: i32 = -32002;
    pub const UNSUPPORTED_COMPONENT: i32 = -32003;
}
