use std::sync::Arc;
use zerolaunch_plugin_api::{
    CachedCandidateData, CandidateId, ScoreBooster, ScoredCandidate, SearchEngine,
};
pub struct SearchPipeline {
    engine: Arc<dyn SearchEngine>,
    boosters: Vec<Arc<dyn ScoreBooster>>,
    top_k: usize,
}

impl SearchPipeline {
    pub fn new(
        engine: Arc<dyn SearchEngine>,
        boosters: Vec<Arc<dyn ScoreBooster>>,
        top_k: usize,
    ) -> Self {
        Self {
            engine,
            boosters,
            top_k,
        }
    }

    /// 执行搜索并截断到 top_k。
    /// 参数：candidates - 候选项缓存；query - 已预处理的查询词。
    /// 返回：按分数降序排列、截断后的 ScoredCandidate 列表。
    pub fn search(&self, candidates: &CachedCandidateData, query: &str) -> Vec<ScoredCandidate> {
        self.search_all(candidates, query)
            .into_iter()
            .take(self.top_k)
            .collect()
    }

    /// 执行全量搜索（不截断 top_k），供调试详情等需要观察完整排序的场景使用。
    /// 参数：candidates - 候选项缓存；query - 已预处理的查询词。
    /// 返回：按分数降序排列的完整 ScoredCandidate 列表。
    pub fn search_all(
        &self,
        candidates: &CachedCandidateData,
        query: &str,
    ) -> Vec<ScoredCandidate> {
        let mut scored = self.engine.calculate_scores(candidates, query);

        for booster in &self.boosters {
            booster.boost(&mut scored, candidates, query);
        }

        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        scored
    }

    /// 获取当前 top_k 值
    pub fn top_k(&self) -> usize {
        self.top_k
    }

    /// 记录候选项被选中启动，通知所有 ScoreBooster 学习用户习惯
    /// 参数：candidate_id - 被选中的候选项 ID；data - 候选项缓存数据；query - 用户查询词
    pub fn record(&self, candidate_id: CandidateId, data: &CachedCandidateData, query: &str) {
        for booster in &self.boosters {
            booster.record(candidate_id, data, query);
        }
    }
}
