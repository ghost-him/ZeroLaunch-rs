use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use tracing::warn;
use zerolaunch_plugin_api::{
    DataSource, KeywordInjector, KeywordOptimizer, ScoreBooster, SearchEngine,
};

use crate::core::config::ConfigManager;

use super::candidate_pipeline::CandidatePipeline;
use super::search_pipeline::SearchPipeline;

/// 插件运行时组件注册中心。
///
/// 集中管理所有领域 trait 对象引用（DataSource、SearchEngine 等），
/// 并提供从注册表按 enabled 状态重建管道的工厂方法。
/// SessionRouter 不再直接持有 5 个 HashMap 字段，而是通过此类间接管理。
pub struct PluginComponentRegistry {
    search_engines: RwLock<HashMap<String, Arc<dyn SearchEngine>>>,
    score_boosters: RwLock<HashMap<String, Arc<dyn ScoreBooster>>>,
    data_sources: RwLock<HashMap<String, Arc<dyn DataSource>>>,
    keyword_optimizers: RwLock<HashMap<String, Arc<dyn KeywordOptimizer>>>,
    keyword_injectors: RwLock<HashMap<String, Arc<dyn KeywordInjector>>>,
}

impl PluginComponentRegistry {
    pub fn new() -> Self {
        Self {
            search_engines: RwLock::new(HashMap::new()),
            score_boosters: RwLock::new(HashMap::new()),
            data_sources: RwLock::new(HashMap::new()),
            keyword_optimizers: RwLock::new(HashMap::new()),
            keyword_injectors: RwLock::new(HashMap::new()),
        }
    }

    /// 注册一个搜索引擎引用，用于配置变更时动态重建管道。
    pub fn register_search_engine(&self, engine: Arc<dyn SearchEngine>) {
        self.search_engines
            .write()
            .insert(engine.component_id().to_string(), engine);
    }

    /// 注册一个分数增强器引用，用于配置变更时动态重建管道。
    pub fn register_score_booster(&self, booster: Arc<dyn ScoreBooster>) {
        self.score_boosters
            .write()
            .insert(booster.component_id().to_string(), booster);
    }

    /// 注册一个数据源引用，供动态启用/禁用使用。
    pub fn register_data_source(&self, source: Arc<dyn DataSource>) {
        self.data_sources
            .write()
            .insert(source.component_id().to_string(), source);
    }

    /// 注册一个关键词优化器引用，供动态启用/禁用使用。
    pub fn register_keyword_optimizer(&self, optimizer: Arc<dyn KeywordOptimizer>) {
        self.keyword_optimizers
            .write()
            .insert(optimizer.component_id().to_string(), optimizer);
    }

    /// 注册一个关键词注入器引用，供动态启用/禁用使用。
    pub fn register_keyword_injector(&self, injector: Arc<dyn KeywordInjector>) {
        self.keyword_injectors
            .write()
            .insert(injector.component_id().to_string(), injector);
    }

    /// 注销一个数据源（按 component_id）。
    pub fn unregister_data_source(&self, component_id: &str) {
        self.data_sources.write().remove(component_id);
    }

    /// 注销一个关键词优化器（按 component_id）。
    pub fn unregister_keyword_optimizer(&self, component_id: &str) {
        self.keyword_optimizers.write().remove(component_id);
    }

    /// 注销一个关键词注入器（按 component_id）。
    pub fn unregister_keyword_injector(&self, component_id: &str) {
        self.keyword_injectors.write().remove(component_id);
    }

    /// 注销一个搜索引擎（按 component_id）。
    pub fn unregister_search_engine(&self, component_id: &str) {
        self.search_engines.write().remove(component_id);
    }

    /// 注销一个分数增强器（按 component_id）。
    pub fn unregister_score_booster(&self, component_id: &str) {
        self.score_boosters.write().remove(component_id);
    }

    /// 检查是否存在指定 ID 的搜索引擎。
    pub fn contains_engine(&self, component_id: &str) -> bool {
        self.search_engines.read().contains_key(component_id)
    }

    /// 当前注册的所有搜索引擎 component_id（供引擎互斥遍历）。
    pub fn search_engine_ids(&self) -> Vec<String> {
        self.search_engines.read().keys().cloned().collect()
    }

    /// 根据当前注册表重建候选管道（仅包含启用的组件）。
    /// 参数：cm - ConfigManager，用于查询 is_enabled 状态。
    pub fn build_candidate_pipeline(&self, cm: &ConfigManager) -> CandidatePipeline {
        let mut pipeline = CandidatePipeline::new();

        // 收集启用的数据源
        for source in self.data_sources.read().values() {
            if cm.is_enabled(source.component_id()) {
                pipeline.add_source(source.clone());
            }
        }

        // 收集启用的关键词优化器
        for optimizer in self.keyword_optimizers.read().values() {
            if cm.is_enabled(optimizer.component_id()) {
                pipeline.add_keyword_optimizer(optimizer.clone());
            }
        }

        // 收集启用的关键词注入器
        for injector in self.keyword_injectors.read().values() {
            if cm.is_enabled(injector.component_id()) {
                pipeline.add_keyword_injector(injector.clone());
            }
        }

        pipeline
    }

    /// 根据当前注册表重建搜索管道（仅包含启用的组件）。
    /// 参数：cm - ConfigManager，用于查询 is_enabled 状态。
    ///       top_k - 搜索结果截断数量。
    /// 返回：如果存在启用的搜索引擎则返回 Some，否则返回 None。
    pub fn build_search_pipeline(
        &self,
        cm: &ConfigManager,
        top_k: usize,
    ) -> Option<SearchPipeline> {
        let engines = self.search_engines.read();
        let mut enabled_engines: Vec<Arc<dyn SearchEngine>> = engines
            .values()
            .filter(|e| cm.is_enabled(e.component_id()))
            .cloned()
            .collect();
        if enabled_engines.len() > 1 {
            warn!(
                "存在 {} 个启用的搜索引擎，按 (priority, component_id) 确定性选取: {:?}",
                enabled_engines.len(),
                enabled_engines
                    .iter()
                    .map(|e| e.component_id())
                    .collect::<Vec<_>>()
            );
        }
        // 确定性选取：priority 小者优先，同优先级按 component_id 字典序（不依赖 HashMap 迭代序）。
        enabled_engines.sort_by(|a, b| {
            a.core()
                .priority()
                .cmp(&b.core().priority())
                .then_with(|| a.component_id().cmp(b.component_id()))
        });

        let boosters = self.score_boosters.read();
        let enabled_boosters: Vec<Arc<dyn ScoreBooster>> = boosters
            .values()
            .filter(|b| cm.is_enabled(b.component_id()))
            .cloned()
            .collect();

        // 无引擎时返回透传管道：候选零分透传，排序完全由增强器决定。
        match enabled_engines.into_iter().next() {
            Some(engine) => Some(SearchPipeline::new(engine, enabled_boosters, top_k)),
            None => Some(SearchPipeline::without_engine(enabled_boosters, top_k)),
        }
    }
}

impl Default for PluginComponentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use zerolaunch_plugin_api::config::{ComponentType, Configurable, SettingDefinition};
    use zerolaunch_plugin_api::{CachedCandidateData, ScoredCandidate};

    struct StubEngine {
        core: zerolaunch_plugin_api::config::ComponentCore,
    }

    impl Configurable for StubEngine {
        fn core(&self) -> &zerolaunch_plugin_api::config::ComponentCore {
            &self.core
        }
        fn setting_schema(&self) -> Vec<SettingDefinition> {
            vec![]
        }
    }

    #[async_trait]
    impl SearchEngine for StubEngine {
        async fn calculate_scores(
            &self,
            _candidates: &CachedCandidateData,
            _query: &str,
        ) -> Vec<ScoredCandidate> {
            vec![]
        }
    }

    fn stub_engine(id: &str, priority: u32) -> Arc<dyn SearchEngine> {
        Arc::new(StubEngine {
            core: zerolaunch_plugin_api::config::ComponentCore::new(
                id.into(),
                id.into(),
                String::new(),
                ComponentType::SearchEngine,
                priority,
            ),
        })
    }

    /// 多引擎启用时确定性选择：priority 小者优先，同 priority 按 component_id 字典序。
    #[test]
    fn test_build_search_pipeline_deterministic_selection() {
        let cm = ConfigManager::new(std::env::temp_dir().join("zl-engine-select-test"));
        let registry = PluginComponentRegistry::new();
        registry.register_search_engine(stub_engine("z-engine", 20));
        registry.register_search_engine(stub_engine("a-engine", 20));
        registry.register_search_engine(stub_engine("b-engine", 10));

        let pipeline = registry
            .build_search_pipeline(&cm, 10)
            .expect("存在启用的搜索引擎");
        assert_eq!(
            pipeline.engine().expect("引擎存在").component_id(),
            "b-engine",
            "priority 最小者优先"
        );

        let registry2 = PluginComponentRegistry::new();
        registry2.register_search_engine(stub_engine("z-engine", 5));
        registry2.register_search_engine(stub_engine("a-engine", 5));
        let pipeline2 = registry2
            .build_search_pipeline(&cm, 10)
            .expect("存在启用的搜索引擎");
        assert_eq!(
            pipeline2.engine().expect("引擎存在").component_id(),
            "a-engine",
            "同 priority 按 component_id 字典序"
        );
    }
}
