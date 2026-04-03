use super::types::DataSource;
use crate::plugin_system::cached_candidate::CachedCandidateData;
use crate::plugin_system::types::KeywordOptimizer;
use std::collections::HashSet;
use std::sync::Arc;

pub struct CandidatePipeline {
    data_sources: Vec<Arc<dyn DataSource>>,
    keyword_optimizers: Vec<Arc<dyn KeywordOptimizer>>,
}

impl CandidatePipeline {
    pub fn new() -> Self {
        Self {
            data_sources: Vec::new(),
            keyword_optimizers: Vec::new(),
        }
    }

    pub fn add_source(&mut self, source: Arc<dyn DataSource>) {
        self.data_sources.push(source);
    }

    pub fn add_keyword_optimizer(&mut self, optimizer: Arc<dyn KeywordOptimizer>) {
        self.keyword_optimizers.push(optimizer);
    }

    pub fn collect(&self) -> CachedCandidateData {
        let mut candidates = CachedCandidateData::new();

        for source in &self.data_sources {
            candidates.add_candidates(source.fetch_candidates());
        }

        let mut sorted_optimizers: Vec<_> = self.keyword_optimizers.iter().collect();
        sorted_optimizers.sort_by_key(|op| op.get_priority());

        for candidate in candidates.get_candidates_mut() {
            let mut accumulated_keywords: Vec<String> = vec![candidate.name.clone()];

            for optimizer in &sorted_optimizers {
                let new_keywords = if optimizer.uses_context() {
                    accumulated_keywords
                        .iter()
                        .flat_map(|kw| optimizer.optimize(kw))
                        .collect()
                } else {
                    optimizer.optimize(&candidate.name)
                };

                accumulated_keywords.extend(new_keywords);
            }

            candidate.keywords = Self::deduplicate_keywords(accumulated_keywords);
        }

        candidates
    }

    fn deduplicate_keywords(keywords: Vec<String>) -> Vec<String> {
        let mut seen = HashSet::new();
        keywords
            .into_iter()
            .filter(|k| seen.insert(k.clone()))
            .collect()
    }
}

impl Default for CandidatePipeline {
    fn default() -> Self {
        Self::new()
    }
}
