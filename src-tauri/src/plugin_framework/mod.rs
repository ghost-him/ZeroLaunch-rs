pub mod builtin;
pub mod builtin_registry;
mod candidate_pipeline;
pub mod component_registry;
mod executor_registry;
pub mod host_handler;
pub mod inspector;
pub mod manager;
pub mod plugin_info;
pub(crate) mod plugin_installer;
pub mod registry;
mod search_pipeline;
mod session_dispatcher;
mod session_state;
pub(crate) mod zlplugin_protocol;

// 类型 re-export（消除冗余 types.rs shim，所有使用者直接从 zerolaunch_plugin_api 导入）
pub use zerolaunch_plugin_api::config::{ComponentType, ConfigError, PathMode, SettingDefinition};
pub use zerolaunch_plugin_api::{
    ActionExecutor, ExecutionContext, ExecutionError, ExecutionTarget, ListItem, Plugin,
    PluginContext, PluginError, PluginMetadata, Query, RegistrationError, ResultAction, TargetType,
};

pub use candidate_pipeline::CandidatePipeline;
pub use executor_registry::ExecutorRegistry;
pub use manager::PluginManagerError;
pub use registry::PluginRegistry;
pub use search_pipeline::*;

// 会话调度与状态（Dispatcher 直接内嵌默认搜索与插件逻辑，无流程抽象层）
pub use session_dispatcher::{
    ConfirmError, ConfirmOutcome, ConfirmRequest, RoutedConfirm, RoutedQuery, SessionDispatcher,
    SessionDispatcherError,
};
pub use session_state::{ActiveSession, PresentationMode, SessionStateEvent};
