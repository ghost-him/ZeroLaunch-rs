use std::collections::HashSet;
use std::sync::Arc;
use zerolaunch_plugin_api::config::Configurable;
use zerolaunch_plugin_api::{CachedCandidateData, DataSource, KeywordOptimizer};

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

    pub fn remove_source(&mut self, component_id: &str) {
        self.data_sources
            .retain(|s| s.component_id() != component_id);
    }

    pub fn add_keyword_optimizer(&mut self, optimizer: Arc<dyn KeywordOptimizer>) {
        self.keyword_optimizers.push(optimizer);
    }

    pub async fn collect(&self) -> CachedCandidateData {
        let mut candidates = CachedCandidateData::new();

        for source in &self.data_sources {
            candidates.add_candidates(source.fetch_candidates().await);
        }

        let mut sorted: Vec<&dyn KeywordOptimizer> =
            self.keyword_optimizers.iter().map(|a| a.as_ref()).collect();
        sorted.sort_by_key(|op| op.get_priority());

        for candidate in candidates.get_candidates_mut() {
            candidate.keywords = Self::apply_keyword_optimizers(&candidate.name, &sorted);
        }

        candidates
    }

    /// 对单个名称运行优化器链，返回去重后的关键字列表。
    /// 参数 `sorted` 必须已按 `get_priority()` 升序排列。
    fn apply_keyword_optimizers(name: &str, sorted: &[&dyn KeywordOptimizer]) -> Vec<String> {
        let mut accumulated: Vec<String> = vec![name.to_string()];
        for optimizer in sorted {
            let new_keywords = if optimizer.uses_context() {
                accumulated
                    .iter()
                    .flat_map(|kw| optimizer.optimize(kw))
                    .collect()
            } else {
                optimizer.optimize(name)
            };
            accumulated.extend(new_keywords);
        }
        Self::deduplicate_keywords(accumulated)
    }

    fn deduplicate_keywords(keywords: Vec<String>) -> Vec<String> {
        let mut seen = HashSet::new();
        keywords
            .into_iter()
            .filter(|k| seen.insert(k.clone()))
            .collect()
    }
    /// 调试用：对单个名称运行关键字优化器链，返回所有生成的关键字。
    /// 不修改候选项缓存。内部自行排序后调用共享逻辑。
    pub fn generate_keywords_for_name(&self, name: &str) -> Vec<String> {
        let mut sorted: Vec<&dyn KeywordOptimizer> =
            self.keyword_optimizers.iter().map(|a| a.as_ref()).collect();
        sorted.sort_by_key(|op| op.get_priority());
        Self::apply_keyword_optimizers(name, &sorted)
    }

    /// 根据 component_id 查找已注册的 Configurable 组件。
    /// 参数：component_id - 组件标识符。
    /// 返回：找到则返回组件引用，否则返回 None。
    pub fn find_configurable(&self, component_id: &str) -> Option<Arc<dyn Configurable>> {
        self.data_sources
            .iter()
            .find(|s| s.component_id() == component_id)
            .map(|s| s.clone() as Arc<dyn Configurable>)
    }
}

impl Default for CandidatePipeline {
    fn default() -> Self {
        Self::new()
    }
}
