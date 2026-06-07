//! ThirdPartyProvider — 第三方插件提供者。
//!
//! 包装 PluginHostManager，管理第三方插件的子进程生命周期、
//! 安装/卸载、崩溃恢复，以及插件发现。

pub mod discovery;
pub mod installer;
pub mod loader;
