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

    // 获取当前所有数据源的候选项，并进行合并去重等处理，最终返回一个包含所有候选项的CachedCandidateData
    pub fn collect(&self) -> CachedCandidateData {
        let mut candidates = CachedCandidateData::new();
        for source in &self.data_sources {
            candidates.add_candidates(source.fetch_candidates());
        }

        // 现在有了初步得到的候选项列表，而这个列表是由 CachedCandidateData 结构体来保证去重的了，所以我们不需要再进行一次去重了，直接对这个候选项列表进行关键字优化就好了

        for optimizer in &self.keyword_optimizer {
            for candidate in candidates.get_candidates_mut() {
                // 根据这个 候选项的名字 来进行关键字优化，得到一个关键字列表，然后把这个关键字列表添加到这个候选项的关键字列表中去
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
