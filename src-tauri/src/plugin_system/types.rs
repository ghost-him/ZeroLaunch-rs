pub use zerolaunch_plugin_api::config::{
    ArrayItem, ArrayUiHint, ComponentType, ConfigActionDef, ConfigError, Configurable,
    DetailActionDef, FieldDefinition, PathMode, PrimitiveType, SettingDefinition, SettingType,
};

// 从 plugin-api 重新导出其他已迁移的类型和组件 trait
pub use zerolaunch_plugin_api::{
    ActionExecutor, CachedCandidateData, CandidateId, ConfirmResult, DataSource, ExecutionContext,
    ExecutionError, ExecutionTarget, KeywordOptimizer, ListItem, Plugin, PluginContext,
    PluginError, PluginMetadata, Query, QueryResponse, RegistrationError, ResultAction,
    ScoreBooster, ScoreDetail, ScoredCandidate, SearchCandidate, SearchEngine, TargetType,
};
