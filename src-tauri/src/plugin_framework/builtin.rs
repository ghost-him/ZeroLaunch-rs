//! 内置组件收集与信息构建。
//!
//! - `make_builtin_info()` — 为标准内置组件创建 PluginInfo

use std::sync::Arc;

use super::plugin_info::PluginInfo;

pub use super::builtin_registry::CollectedBuiltins;

/// 为标准内置组件创建 PluginInfo。
pub fn make_builtin_info(
    configurable: &Arc<dyn zerolaunch_plugin_api::config::Configurable>,
    enabled: bool,
) -> PluginInfo {
    PluginInfo::builtin(
        configurable.component_id(),
        configurable.component_name(),
        1,
        enabled,
        configurable.priority(),
    )
}
