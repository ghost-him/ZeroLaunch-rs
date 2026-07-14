use std::sync::Arc;
use zerolaunch_plugin_api::config::Configurable;
use zerolaunch_plugin_api::{CachedCandidateData, DataSource, KeywordInjector, KeywordOptimizer};

pub struct CandidatePipeline {
    data_sources: Vec<Arc<dyn DataSource>>,
    keyword_optimizers: Vec<Arc<dyn KeywordOptimizer>>,
    keyword_injectors: Vec<Arc<dyn KeywordInjector>>,
}

impl CandidatePipeline {
    pub fn new() -> Self {
        Self {
            data_sources: Vec::new(),
            keyword_optimizers: Vec::new(),
            keyword_injectors: Vec::new(),
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

    pub fn remove_keyword_optimizer(&mut self, component_id: &str) {
        self.keyword_optimizers
            .retain(|op| op.component_id() != component_id);
    }

    pub fn add_keyword_injector(&mut self, injector: Arc<dyn KeywordInjector>) {
        self.keyword_injectors.push(injector);
    }

    pub fn remove_keyword_injector(&mut self, component_id: &str) {
        self.keyword_injectors
            .retain(|inj| inj.component_id() != component_id);
    }

    /// 收集候选项。
    /// 管道中只包含已启用的组件（由 SessionRouter 在启用/禁用时动态维护），
    /// 此处无需再做 enabled 过滤。
    pub async fn collect(&self) -> CachedCandidateData {
        let mut candidates = CachedCandidateData::new();

        for source in &self.data_sources {
            candidates.add_candidates(source.fetch_candidates().await);
        }

        // 排序优化器
        let mut sorted: Vec<&dyn KeywordOptimizer> =
            self.keyword_optimizers.iter().map(|a| a.as_ref()).collect();
        sorted.sort_by_key(|op| op.get_priority());

        // 注入器无需排序
        let injectors: Vec<&dyn KeywordInjector> =
            self.keyword_injectors.iter().map(|a| a.as_ref()).collect();

        for candidate in candidates.get_candidates_mut() {
            // 1. 名称无关的字符串变换
            candidate.keywords = Self::apply_keyword_optimizers(&candidate.name, &sorted);

            // 2. 候选项相关的关键字注入
            for injector in &injectors {
                candidate
                    .keywords
                    .extend(injector.inject_keywords(candidate));
            }

            candidate.keywords = Self::deduplicate_keywords(candidate.keywords.clone());
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
        let mut seen = std::collections::HashSet::new();
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
        // 先从数据源中查找
        if let Some(found) = self
            .data_sources
            .iter()
            .find(|s| s.component_id() == component_id)
            .map(|s| s.clone() as Arc<dyn Configurable>)
        {
            return Some(found);
        }
        // 再从关键词优化器中查找
        if let Some(found) = self
            .keyword_optimizers
            .iter()
            .find(|op| op.component_id() == component_id)
            .map(|op| op.clone() as Arc<dyn Configurable>)
        {
            return Some(found);
        }
        // 最后从关键词注入器中查找
        self.keyword_injectors
            .iter()
            .find(|inj| inj.component_id() == component_id)
            .map(|inj| inj.clone() as Arc<dyn Configurable>)
    }
}

impl Default for CandidatePipeline {
    fn default() -> Self {
        Self::new()
    }
}
