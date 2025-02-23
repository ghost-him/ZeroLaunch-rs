use std::error::Error as StdError;
use std::fmt;
use tauri_plugin_autostart::Error as AutostartError;

#[derive(Debug)]
pub enum AppError {
    /// 资源未初始化错误
    NotInitialized {
        resource: String,
        context: Option<String>,
    },

    /// 锁相关错误
    LockError {
        lock_type: String,
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// 自动启动错误
    AutostartError(AutostartError),

    /// 配置错误
    ConfigError { section: String, detail: String },

    /// IO错误
    IoError(std::io::Error),

    /// 序列化/反序列化错误
    SerdeError(serde_json::Error),

    /// 通用错误容器
    Custom { message: String, code: u32 },
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::NotInitialized { resource, context } => {
                let ctx = context.as_deref().unwrap_or("no additional context");
                write!(
                    f,
                    "Resource '{}' not initialized. Context: {}",
                    resource, ctx
                )
            }
            AppError::LockError { lock_type, source } => {
                if let Some(err) = source {
                    write!(f, "LockError on {}: {}", lock_type, err)
                } else {
                    write!(f, "LockError on {}: unknown cause", lock_type)
                }
            }
            AppError::AutostartError(e) => write!(f, "AutostartError: {}", e),
            AppError::ConfigError { section, detail } => {
                write!(f, "ConfigError in [{}]: {}", section, detail)
            }
            AppError::IoError(e) => write!(f, "IOError: {}", e),
            AppError::SerdeError(e) => write!(f, "SerdeError: {}", e),
            AppError::Custom { message, code } => {
                write!(f, "CustomError[{}]: {}", code, message)
            }
        }
    }
}

impl StdError for AppError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            AppError::LockError { source, .. } => source.as_deref().map(|e| e as &dyn StdError),
            AppError::AutostartError(e) => Some(e),
            AppError::IoError(e) => Some(e),
            AppError::SerdeError(e) => Some(e),
            _ => None,
        }
    }
}

// 转换实现
impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::IoError(err)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::SerdeError(err)
    }
}

impl From<AutostartError> for AppError {
    fn from(err: AutostartError) -> Self {
        AppError::AutostartError(err)
    }
}

// 实用方法
impl AppError {
    pub fn not_initialized(resource: &str) -> Self {
        AppError::NotInitialized {
            resource: resource.to_string(),
            context: None,
        }
    }

    pub fn with_context(mut self, context: &str) -> Self {
        if let AppError::NotInitialized { context: ctx, .. } = &mut self {
            *ctx = Some(context.to_string());
        }
        self
    }

    pub fn lock_error(lock_type: &str, source: Option<Box<dyn StdError + Send + Sync>>) -> Self {
        AppError::LockError {
            lock_type: lock_type.to_string(),
            source,
        }
    }
}
