pub mod cached_candidate;
pub mod plugin_trait;
pub mod types;

pub use cached_candidate::CachedCandidateData;
pub use plugin_trait::Plugin;
pub use types::{
    ActionExecutor, CandidateId, DataSource, ExecutionContext, ExecutionError, ExecutionTarget,
    KeywordInjector, KeywordOptimizer, ListItem, PanelInteraction, PanelKeyAction, PanelKeyBinding,
    PanelQueryTrigger, PluginContext, PluginError, PluginKind, PluginMetadata, Query, QueryChannel,
    QueryResponse, QueryRevisionGate, RegistrationError, ResultAction, ScoreBooster, ScoreDetail,
    ScoreDetailKind, ScoredCandidate, SearchCandidate, SearchEngine, TargetType,
};
