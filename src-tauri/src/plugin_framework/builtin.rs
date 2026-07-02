//! 内置组件收集与信息构建。
//!
//! - `make_builtin_info()` — 为标准内置组件创建 PluginInfo

use std::sync::Arc;

use super::plugin_info::{PluginInfo, PluginKind, PluginStatus};

pub use super::builtin_registry::CollectedBuiltins;

/// 为标准内置组件创建 PluginInfo。
pub fn make_builtin_info(
    configurable: &Arc<dyn zerolaunch_plugin_api::config::Configurable>,
    enabled: bool,
) -> PluginInfo {
    PluginInfo {
        id: configurable.component_id().to_string(),
        name: configurable.component_name().to_string(),
        kind: PluginKind::Builtin,
        status: PluginStatus::Active,
        version: None,
        description: None,
        author: Some("ZeroLaunch".to_string()),
        component_count: 1,
        enabled,
    }
}
