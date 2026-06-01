// 从 core/types 重新导出基础类型，保证现有代码的兼容性
pub use crate::core::types::*;

// 从 plugin-api 重新导出所有已迁移的类型和组件 trait
pub use zerolaunch_plugin_api::{
    ActionExecutor, CachedCandidateData, CandidateId, ConfirmResult, DataSource, ExecutionContext,
    ExecutionError, ExecutionTarget, KeywordOptimizer, ListItem, Plugin, PluginContext,
    PluginError, PluginMetadata, Query, QueryResponse, RegistrationError, ResultAction,
    ScoreBooster, ScoreDetail, ScoredCandidate, SearchCandidate, SearchEngine, TargetType,
};
