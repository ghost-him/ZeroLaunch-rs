pub mod cached_candidate;
mod candidate_pipeline;
mod dispatcher;
mod executor_registry;
mod registry;
mod search_pipeline;
pub mod service;
mod session_router;
pub mod types;

pub use crate::core::types::DetailActionDef;
pub use candidate_pipeline::CandidatePipeline;
pub use dispatcher::QueryDispatcher;
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
