//! 核心配置组件注册条目。
//!
//! `CoreComponentEntry` 被 `inventory` 用于编译期收集核心配置组件，
//! 由 `core/config/components/*.rs` 中的 `inventory::submit!` 提交，
//! 由 `plugin_system/builtin_registry.rs` 中的收集器遍历构造。
//!
//! 此类型放在 `core/` 层是因为它是 config component 的注册契约，
//! 避免 `core/config/components/*.rs` 反向依赖 `plugin_system/`。

use crate::sdk::HostApi;
use std::sync::Arc;
use zerolaunch_plugin_api::config::Configurable;

/// 核心配置组件的工厂函数签名。
pub type CoreComponentFactory = fn(Arc<HostApi>) -> Arc<dyn Configurable>;

/// 核心配置组件条目。
pub struct CoreComponentEntry {
    pub component_id: &'static str,
    pub priority: u32,
    pub factory: CoreComponentFactory,
}

::inventory::collect!(CoreComponentEntry);
