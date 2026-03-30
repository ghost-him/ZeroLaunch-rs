use super::types::*;
use crate::plugin_system::cached_candidate::CachedCandidateData;
use std::sync::Arc;
pub struct SearchPipeline {
    engine: Option<Arc<dyn SearchEngine>>,
    boosters: Vec<Arc<dyn ScoreBooster>>,
    top_k: usize,
}

impl SearchPipeline {
    pub fn new(
        engine: Option<Arc<dyn SearchEngine>>,
        boosters: Vec<Arc<dyn ScoreBooster>>,
        top_k: usize,
    ) -> Self {
        Self {
            engine,
            boosters,
            top_k,
        }
    }

    pub fn search(&self, candidates: &CachedCandidateData, query: &str) -> Vec<ScoredCandidate> {
        let mut scored = self
            .engine
            .as_ref()
            .unwrap()
            .calculate_scores(candidates, query);

        if scored.is_empty() {
            return Vec::new();
        }

        for booster in &self.boosters {
            booster.boost(&mut scored);
        }

        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        scored.into_iter().take(self.top_k).collect()
    }
}
