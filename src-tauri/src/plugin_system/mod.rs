mod api;
pub mod cached_candidate;
mod candidate_pipeline;
mod dispatcher;
mod launcher_registry;
mod registry;
mod search_pipeline;
pub mod service;
mod session_router;
pub mod types;

pub use api::DefaultPluginAPI;
pub use candidate_pipeline::CandidatePipeline;
pub use dispatcher::QueryDispatcher;
pub use launcher_registry::LauncherRegistry;
pub use registry::PluginRegistry;
pub use search_pipeline::*;
pub use service::PluginService;
pub use session_router::{SessionMode, SessionRouter};
pub use types::{
    ComponentType, ConfigError, Configurable, LaunchError, LaunchMethod, LaunchMethodType,
    Launcher, ListItem, LogLevel, PathMode, Plugin, PluginAPI, PluginContext, PluginError,
    PluginMetadata, Query, ResultAction, SettingDefinition, SettingType,
};
