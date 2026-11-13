//! ZeroLaunch 后端库：配置、插件、搜索会话、CLI 服务与平台装配。
//!
//! 窗口、托盘与输入链路由 `zerolaunch-gpui` 外壳承载；本 crate 不依赖任何 UI 框架。

pub mod builtin_plugin;
pub mod cli_server;
pub mod core;
pub mod kernel;
pub mod logging;
pub mod plugin_framework;
pub mod plugin_market;
pub mod runtime;
pub mod sdk;
pub mod state;
pub mod updater;
pub mod utils;
pub mod window;
