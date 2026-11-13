mod app_executor;
mod builtin_command_executor;
mod command_executor;
mod file_executor;
mod path_executor;
mod url_executor;
mod window_activate_executor;

pub use app_executor::AppExecutor;
pub use builtin_command_executor::BuiltinCommandExecutor;
pub use command_executor::CommandExecutor;
pub use file_executor::FileExecutor;
pub use path_executor::PathExecutor;
pub use url_executor::UrlExecutor;
pub use window_activate_executor::WindowActivateExecutor;

/// 内置执行器共享的默认动作 ID（打开/执行）。
/// 属于宿主内置实现，不进 plugin-api：第三方执行器直接使用 SDK trait 的默认动作实现。
pub(crate) const ACTION_EXECUTE: &str = "execute";

/// 窗口唤醒动作 ID —— 由 `window-activate-executor` 声明，确认路由改写时消费。
pub(crate) use window_activate_executor::ACTION_ACTIVATE_WINDOW;
