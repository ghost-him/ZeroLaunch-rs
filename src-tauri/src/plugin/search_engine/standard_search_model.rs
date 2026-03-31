use crate::plugin_system::cached_candidate::CachedCandidateData;
use crate::plugin_system::types::{ComponentType, Configurable, ScoredCandidate, SearchEngine};

#[allow(dead_code)]
pub struct StandardSearchModel {}

impl Configurable for StandardSearchModel {
    fn component_id(&self) -> &str {
        "standard-search-model"
    }

    fn component_name(&self) -> &str {
        "标准搜索引擎"
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::SearchEngine
    }
}

impl SearchEngine for StandardSearchModel {
    fn calculate_scores(
        &self,
        candidates: &CachedCandidateData,
        _query: &str,
    ) -> Vec<ScoredCandidate> {
        candidates
            .get_candidates()
            .iter()
            .map(|c| ScoredCandidate {
                candidate_id: c.id,
                score: 0.0,
                detailed_score: Vec::new(),
            })
            .collect()
    }
}
