//! PluginManager — 插件身份与生命周期的唯一权威来源。
//!
//! PluginManager 知道「有哪些插件」「如何安装/卸载」「如何启动/停止」「崩溃恢复」。
//! 它把插件各部分（Configurable、DataSource、Executor、Plugin trait 对象）
//! 注册到 ConfigManager 和 SessionRouter，但这两个管理器不需要知道「这是一个插件」。
//!
//! # 子模块
//! - `types` — PluginInfo, PluginKind, PluginStatus 等核心数据类型
//! - `manager` — PluginManager struct，统一入口
//! - `builtin` — BuiltinProvider，包装 inventory 收集 + 注册
//! - `third_party` — ThirdPartyProvider，包装 PluginHostManager + discovery + installer

pub mod builtin;
pub mod manager;
pub mod third_party;
pub mod types;
