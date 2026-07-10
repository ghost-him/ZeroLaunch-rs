pub mod builtin;
pub mod builtin_registry;
mod candidate_pipeline;
mod executor_registry;
pub mod host_handler;
pub mod inspector;
pub mod manager;
pub mod plugin_info;
pub(crate) mod plugin_installer;
mod registry;
mod search_pipeline;
pub mod service;
mod session_router;
pub(crate) mod zlplugin_protocol;

// 类型 re-export（消除冗余 types.rs shim，所有使用者直接从 zerolaunch_plugin_api 导入）
pub use zerolaunch_plugin_api::config::{
    ComponentType, ConfigError, DetailActionDef, PathMode, SettingDefinition, SettingType,
};
pub use zerolaunch_plugin_api::{
    ActionExecutor, ExecutionContext, ExecutionError, ExecutionTarget, ListItem, Plugin,
    PluginContext, PluginError, PluginMetadata, Query, RegistrationError, ResultAction, TargetType,
};

pub use candidate_pipeline::CandidatePipeline;
pub use executor_registry::ExecutorRegistry;
pub use manager::PluginManagerError;
pub use registry::PluginRegistry;
pub use search_pipeline::*;
pub use service::PluginService;
pub use session_router::{SessionMode, SessionRouter, SessionRouterError};
