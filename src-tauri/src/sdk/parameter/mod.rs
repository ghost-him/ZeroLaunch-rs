//! 参数解析模块
//!
//! 负责解析和填充启动模板中的参数占位符，支持以下参数类型：
//! - `{}` - 用户提供的位置参数
//! - `{clip}` - 剪贴板内容
//! - `{hwnd}` - 当前活动窗口句柄
//! - `{selection}` - 唤醒前活动窗口的选中文本

pub mod default_resolver;
pub mod provider;
pub mod resolver;
pub mod template_parser;
pub mod types;

pub use default_resolver::DefaultParameterResolver;
pub use provider::SystemParameterProvider;
pub use resolver::ParameterResolver;
pub use types::{ParameterError, ParameterSnapshot};
