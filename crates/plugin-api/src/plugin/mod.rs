pub mod cached_candidate;
pub mod plugin_trait;
pub mod types;

pub use cached_candidate::CachedCandidateData;
pub use plugin_trait::Plugin;
pub use types::{
    ActionExecutor, CandidateId, ConfirmResult, DataSource, ExecutionContext, ExecutionError,
    ExecutionTarget, KeywordInjector, KeywordOptimizer, ListItem, PanelInteraction,
    PanelQueryTrigger, PluginContext, PluginError, PluginMetadata, Query, QueryResponse,
    QueryRevisionGate, RegistrationError, ResultAction, ScoreBooster, ScoreDetail, ScoredCandidate,
    SearchCandidate, SearchEngine, TargetType,
};
