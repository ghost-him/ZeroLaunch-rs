//! 参数解析器模块
//!
//! 负责解析和填充启动模板中的参数占位符
//! 支持以下参数类型：
//! - `{}` - 用户提供的位置参数
//! - `{clip}` - 剪贴板内容
//! - `{hwnd}` - 当前活动窗口句柄

pub mod parameter_types;
pub mod providers;
pub mod resolver;
pub mod template_parser;

pub use parameter_types::{ParameterType, SystemParameter};
pub use providers::{ClipboardProvider, WindowHandleProvider};
pub use resolver::{ParameterResolver, ResolverError, SystemParameterSnapshot};
pub use template_parser::{Parameter, TemplateParser};
