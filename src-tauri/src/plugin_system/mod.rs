pub mod adapter_registrar;
pub mod builtin;
pub mod builtin_registry;
pub mod cached_candidate;
mod candidate_pipeline;
mod executor_registry;
pub mod host_handler;
#[cfg(feature = "inspector")]
pub mod inspector;
pub mod manager;
pub mod plugin_info;
mod registry;
mod search_pipeline;
pub mod service;
mod session_router;
pub mod types;

pub use crate::core::types::DetailActionDef;
pub use adapter_registrar::{
    register_builtin_collected, AdapterRegistrar, DefaultAdapterRegistrar,
};
pub use candidate_pipeline::CandidatePipeline;
pub use executor_registry::ExecutorRegistry;
pub use registry::PluginRegistry;
pub use search_pipeline::*;
pub use service::PluginService;
pub use session_router::{SessionMode, SessionRouter};
pub use types::{
    ActionExecutor, ComponentType, ConfigActionDef, ConfigError, Configurable, ExecutionContext,
    ExecutionError, ExecutionTarget, ListItem, PathMode, Plugin, PluginContext, PluginError,
    PluginMetadata, Query, RegistrationError, ResultAction, SettingDefinition, SettingType,
    TargetType,
};
