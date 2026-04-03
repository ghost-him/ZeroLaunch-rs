use super::types::DataSource;
use crate::plugin_system::cached_candidate::CachedCandidateData;
use crate::plugin_system::types::KeywordOptimizer;
use std::sync::Arc;

pub struct CandidatePipeline {
    data_sources: Vec<Arc<dyn DataSource>>,
    keyword_optimizer: Vec<Arc<dyn KeywordOptimizer>>,
}

impl CandidatePipeline {
    pub fn new() -> Self {
        Self {
            data_sources: Vec::new(),
            keyword_optimizer: Vec::new(),
        }
    }

    pub fn add_source(&mut self, source: Arc<dyn DataSource>) {
        self.data_sources.push(source);
    }

    pub fn add_keyword_optimizer(&mut self, optimizer: Arc<dyn KeywordOptimizer>) {
        self.keyword_optimizer.push(optimizer);
    }

    pub fn collect(&self) -> CachedCandidateData {
        let mut candidates = CachedCandidateData::new();
        for source in &self.data_sources {
            candidates.add_candidates(source.fetch_candidates());
        }

        for optimizer in &self.keyword_optimizer {
            for candidate in candidates.get_candidates_mut() {
                let keywords = optimizer.optimize(&candidate.name);
                candidate.keywords.extend(keywords);
            }
        }

        candidates
    }
}

impl Default for CandidatePipeline {
    fn default() -> Self {
        Self::new()
    }
}
