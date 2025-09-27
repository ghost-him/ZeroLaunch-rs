use std::error::Error as StdError;
use std::fmt;
use tauri_plugin_autostart::Error as AutostartError;
use thiserror::Error;

// --- 核心错误类型定义 ---

/// 应用程序统一错误类型。
/// 设计哲学:
/// 1.  **明确分类**: 错误被分为多种类型，但核心是区分 `ProgrammingError` 和其他可恢复的错误。
/// 2.  **`ProgrammingError` 表示逻辑Bug**: 任何`ProgrammingError`都代表代码中存在一个需要修复的bug。
///     遇到这类错误时，程序应该立即 `panic`，而不是尝试恢复。
/// 3.  **使用 `?` 操作符**: 对于可恢复的错误，函数应该返回 `AppResult<T>`，并使用 `?` 操作符来传播错误。
/// 4.  **拥抱标准库**: 放弃自定义的 `safe_unwrap!` 等宏，转而使用 `Option::ok_or_else` 和 `Result::map_err`，
///     这是更标准、更灵活的 Rust 实践。
/// 5.  **使用 `expect_programming`**: 对于那些你断定“绝不应该失败”的操作，使用我们提供的
///     `ResultExt` 和 `OptionExt` trait 中的 `.expect_programming("...")` 方法。
///     这会清晰地表明失败是一个逻辑错误，并立即 `panic`。
#[derive(Debug, Error)]
pub enum AppError {
    /// 程序员错误 - **这应该总是导致 `panic`**。
    /// 这类错误表示程序逻辑有误，例如不变量被违反、状态不一致等。
    /// 它应该通过修改代码来修复，而不是在运行时处理。
    #[error("程序逻辑错误: {message}。这是一个需要修复的BUG。")]
    ProgrammingError { message: String },

    /// 锁相关错误
    #[error("锁错误 {lock_type}: {message}")]
    LockError {
        lock_type: String,
        message: String,
        #[source]
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// 配置错误 - 用户配置问题
    #[error("配置错误 [{section}]: {detail}")]
    ConfigError { section: String, detail: String },

    /// 网络相关错误
    #[error("网络错误: {message}")]
    NetworkError {
        message: String,
        #[source]
        source: Option<Box<dyn StdError + Send + Sync>>,
    },

    /// 文件系统错误
    #[error("文件系统错误: {message}{}", path.as_ref().map(|p| format!(" (路径: {})", p)).unwrap_or_default())]
    FileSystemError {
        message: String,
        path: Option<String>,
        #[source]
        source: Option<std::io::Error>,
    },

    // ... 其他特定领域的错误可以继续添加 ...
    #[error("窗口操作错误: {message}")]
    WindowError { message: String },

    #[error("快捷键错误: {message}")]
    ShortcutError { message: String },

    #[error("存储错误: {message}")]
    StorageError { message: String },

    /// 自动启动错误
    #[error("自动启动错误: {0}")]
    AutostartError(#[from] AutostartError),

    /// IO错误
    #[error("IO错误: {0}")]
    IoError(#[from] std::io::Error),

    /// 序列化/反序列化错误
    #[error("序列化错误: {0}")]
    SerdeError(#[from] serde_json::Error),

    /// ONNX Runtime错误 - AI模型相关
    #[cfg(feature = "ai")]
    #[error("AI模型错误: {0}")]
    OrtError(#[from] ort::Error),

    /// 图片处理错误
    #[error("图片处理错误: {message}")]
    ImageProcessingError { message: String },

    /// 通用错误容器，用于其他未明确分类的错误
    #[error("错误[{code}]: {message}")]
    Custom { message: String, code: u32 },
}

/// 应用程序专用的 `Result` 类型别名。
pub type AppResult<T> = Result<T, AppError>;

// --- 错误构造与辅助方法 ---

impl AppError {
    /// 创建一个程序员错误。
    /// 调用这个函数意味着你发现了一个代码逻辑上的 bug。
    pub fn programming_error(message: impl Into<String>) -> Self {
        Self::ProgrammingError {
            message: message.into(),
        }
    }

    /// 用于 `Option::ok_or_else`，当 `None` 是一个逻辑错误时。
    pub fn unwrap_failed(context: impl Into<String>) -> Self {
        Self::programming_error(format!("unwrap失败(Option为None): {}", context.into()))
    }

    /// 创建一个带源错误的网络错误。
    pub fn network_error_with_source(
        message: impl Into<String>,
        source: impl Into<Box<dyn StdError + Send + Sync>>,
    ) -> Self {
        Self::NetworkError {
            message: message.into(),
            source: Some(source.into()),
        }
    }

    /// 创建一个带IO源错误的文件系统错误。
    pub fn filesystem_error_with_io(
        message: impl Into<String>,
        path: Option<String>,
        source: std::io::Error,
    ) -> Self {
        Self::FileSystemError {
            message: message.into(),
            path,
            source: Some(source),
        }
    }

    /// 创建一个自定义错误。
    pub fn custom(message: impl Into<String>, code: u32) -> Self {
        Self::Custom {
            message: message.into(),
            code,
        }
    }

    /// 检查一个错误是否是程序员错误。
    /// 这在顶层错误处理器中可能有用，比如决定是记录错误并继续，还是直接终止进程。
    pub fn is_programming_error(&self) -> bool {
        matches!(self, AppError::ProgrammingError { .. })
    }
}

// --- 扩展 Trait，用于处理程序员错误 ---

/// 为 `Result` 类型提供处理程序员错误的扩展方法。
pub trait ResultExt<T> {
    /// 当 `Result` 为 `Err` 时，将其视为一个程序员错误并 `panic`。
    ///
    /// 这等同于标准库的 `.expect()`，但用于强调这是一个业务逻辑上的断言失败。
    ///
    /// # Panics
    ///
    /// 如果 `self` 是 `Err`，则会 panic。
    ///
    /// # 示例
    /// ```
    /// # use crate::error::ResultExt; // 假设 trait 在你的 crate::error 模块中
    /// let result: Result<i32, &str> = Ok(10);
    /// let value = result.expect_programming("关键计算不应失败");
    /// ```
    fn expect_programming(self, msg: &str) -> T;
}

impl<T, E> ResultExt<T> for Result<T, E>
where
    E: fmt::Debug,
{
    #[inline]
    #[track_caller] // 这会让 panic 信息指向调用 expect_programming 的地方，而不是这里
    fn expect_programming(self, msg: &str) -> T {
        match self {
            Ok(t) => t,
            Err(e) => {
                panic!("程序逻辑错误: {}. (原始错误: {:?})", msg, e);
            }
        }
    }
}

/// 为 `Option` 类型提供处理程序员错误的扩展方法。
pub trait OptionExt<T> {
    /// 当 `Option` 为 `None` 时，将其视为一个程序员错误并 `panic`。
    ///
    /// 这等同于标准库的 `.expect()`，但用于强调这是一个业务逻辑上的断言失败。
    ///
    /// # Panics
    ///
    /// 如果 `self` 是 `None`，则会 panic。
    ///
    /// # 示例
    /// ```
    /// # use crate::error::OptionExt; // 假设 trait 在你的 crate::error 模块中
    /// let option: Option<i32> = Some(10);
    /// let value = option.expect_programming("在初始化后，该值必须存在");
    /// ```
    fn expect_programming(self, msg: &str) -> T;
}

impl<T> OptionExt<T> for Option<T> {
    #[inline]
    #[track_caller]
    fn expect_programming(self, msg: &str) -> T {
        match self {
            Some(t) => t,
            None => {
                panic!("程序逻辑错误: {}. (Option值为None)", msg);
            }
        }
    }
}

// --- 用法示例 ---

#[allow(dead_code)]
fn example_usage(config_value: Option<&str>) -> AppResult<String> {
    // 场景1: Option<T> -> Result<T, AppError>
    // 如果 config_value 是 None，我们认为这是一个可恢复的配置错误。
    let value = config_value.ok_or_else(|| AppError::ConfigError {
        section: "example".to_string(),
        detail: "缺少必要的配置值".to_string(),
    })?;

    // 场景2: Result<T, E> -> Result<T, AppError>
    // 使用 `?` 和 `#[from]` 自动转换 `std::io::Error`
    let content = std::fs::read_to_string("some_file.txt")?;

    // 场景3: 处理一个你断定不应失败的操作
    // 假设我们从一个刚插入的数据库记录中获取ID，它必须存在。
    // 引入 trait 来使用新方法。
    use crate::error::OptionExt; // 模块路径根据你的项目结构调整
    let new_record_id: Option<i64> = Some(123); // 模拟数据库返回
    let _id = new_record_id.expect_programming("刚插入的记录必须有ID");
    // 如果 new_record_id 是 None，上面这行会 panic。

    Ok(format!("{} - {}", value, content))
}
