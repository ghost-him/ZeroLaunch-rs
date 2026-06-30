pub mod builtin;
pub mod builtin_registry;
mod candidate_pipeline;
mod executor_registry;
pub mod host_handler;
#[cfg(feature = "inspector")]
pub mod inspector;
pub mod manager;
pub mod plugin_info;
pub(crate) mod plugin_installer;
mod registry;
mod search_pipeline;
pub mod service;
mod session_router;
pub mod types;
pub(crate) mod zlplugin_protocol;

pub use candidate_pipeline::CandidatePipeline;
pub use executor_registry::ExecutorRegistry;
pub use registry::PluginRegistry;
pub use search_pipeline::*;
pub use service::PluginService;
pub use session_router::{SessionMode, SessionRouter};
pub use types::{
    ActionExecutor, CachedCandidateData, ComponentType, ConfigActionDef, ConfigError, Configurable,
    DetailActionDef, ExecutionContext, ExecutionError, ExecutionTarget, ListItem, PathMode, Plugin,
    PluginContext, PluginError, PluginMetadata, Query, RegistrationError, ResultAction,
    SettingDefinition, SettingType, TargetType,
};
