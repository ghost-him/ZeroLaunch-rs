use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
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

    /// 检查是否存在指定 ID 的搜索引擎。
    pub fn contains_engine(&self, component_id: &str) -> bool {
        self.search_engines.read().contains_key(component_id)
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
        let enabled_engine = engines
            .values()
            .find(|e| cm.is_enabled(e.component_id()))
            .cloned();

        let boosters = self.score_boosters.read();
        let enabled_boosters: Vec<Arc<dyn ScoreBooster>> = boosters
            .values()
            .filter(|b| cm.is_enabled(b.component_id()))
            .cloned()
            .collect();

        enabled_engine.map(|engine| SearchPipeline::new(engine, enabled_boosters, top_k))
    }
}

impl Default for PluginComponentRegistry {
    fn default() -> Self {
        Self::new()
    }
}
