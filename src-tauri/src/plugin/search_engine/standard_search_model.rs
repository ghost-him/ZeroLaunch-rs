use crate::plugin_system::cached_candidate::CachedCandidateData;
use crate::plugin_system::types::ScoredCandidate;
use crate::plugin_system::types::SearchEngine;

#[allow(dead_code)]
pub struct StandardSearchModel {}

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
