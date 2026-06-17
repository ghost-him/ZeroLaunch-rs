//! AdapterRegistrar trait 和默认实现。
//!
//! 定义第三方插件适配器的注册/解注册接口，将 PluginManager 与
//! ConfigManager / SessionRouter 解耦。同时提供内置组件注册方法。

use async_trait::async_trait;
use std::sync::Arc;
use zerolaunch_plugin_host::manager::RegisteredAdapters;

use crate::core::config::ConfigManager;
use crate::plugin_system::SessionRouter;

use super::builtin_registry::CollectedBuiltins;
use super::types::{DataSource, KeywordOptimizer};

// ── AdapterRegistrar trait ──────────────────────────────────────────

/// 插件适配器的注册/解注册接口。
///
/// 由调用方实现并注入 PluginManager，使 PluginManager 无需直接依赖
/// ConfigManager / SessionRouter。注册逻辑集中在一处，正常加载与
/// 崩溃恢复复用同一实现。
///
/// 注册不自动刷新候选项缓存；调用方应在批量注册完成后
/// 显式调用 [`refresh()`](AdapterRegistrar::refresh) 以一次性重建候选项缓存。
#[async_trait]
pub trait AdapterRegistrar: Send + Sync {
    /// 注册第三方插件适配器到各子系统（ConfigManager + SessionRouter）。
    async fn register(&self, adapters: &RegisteredAdapters);

    /// 从各子系统解注册第三方插件适配器。
    async fn unregister(&self, adapters: &RegisteredAdapters);

    /// 将已收集的内置组件注册到 ConfigManager / SessionRouter。
    ///
    /// 注册规则：
    /// - Configurable → ConfigManager::register（所有组件）
    /// - ActionExecutor → SessionRouter::register_executor
    /// - DataSource/KeywordOptimizer → 仅注册 Configurable，trait 对象由调用方管理
    /// - SearchEngine → SessionRouter::register_search_engine
    /// - ScoreBooster → SessionRouter::register_score_booster
    /// - Plugin → SessionRouter::plugin_service().register
    /// - CoreComponent → 仅 ConfigManager::register
    ///
    /// 返回 DataSource 和 KeywordOptimizer trait 对象列表，
    /// 供调用方构建 CandidatePipeline。
    fn register_builtin(&self, collected: &CollectedBuiltins) -> BuiltinPipelineEntries;

    /// 刷新候选项缓存（第三方插件注册/解注册后显式调用）。
    async fn refresh(&self);
}

// ── BuiltinPipelineEntries ──────────────────────────────────────────

/// 注册内置组件后返回的管道入口：DataSources + KeywordOptimizers。
pub type BuiltinPipelineEntries = (Vec<Arc<dyn DataSource>>, Vec<Arc<dyn KeywordOptimizer>>);

// ── DefaultAdapterRegistrar ─────────────────────────────────────────

/// AdapterRegistrar 的默认实现，持有 ConfigManager / SessionRouter 引用。
///
/// 注册不自动刷新候选项缓存；调用方应在批量注册完成后
/// 显式调用 `refresh()` 以一次性重建候选项缓存。
pub struct DefaultAdapterRegistrar {
    config_manager: Arc<ConfigManager>,
    session_router: Arc<SessionRouter>,
}

impl DefaultAdapterRegistrar {
    /// 创建 DefaultAdapterRegistrar。
    pub fn new(config_manager: Arc<ConfigManager>, session_router: Arc<SessionRouter>) -> Self {
        Self {
            config_manager,
            session_router,
        }
    }
}

#[async_trait]
impl AdapterRegistrar for DefaultAdapterRegistrar {
    async fn register(&self, adapters: &RegisteredAdapters) {
        let cm = self.config_manager.clone();
        let sr = self.session_router.clone();
        let configs = adapters.configurables.clone();
        let sources = adapters.data_sources.clone();
        let executors_list = adapters.executors.clone();
        let plugin_opt = adapters.plugin.clone();

        // 注册 Configurable 到 ConfigManager
        for c in &configs {
            cm.register(c.clone());
        }
        // 注册 DataSource 到 SessionRouter
        for ds in &sources {
            sr.register_data_source(ds.clone()).await;
        }
        // 注册 Executor 到 SessionRouter
        for ex in &executors_list {
            sr.register_executor(ex.clone());
        }
        // 注册远程 Plugin 到 SessionRouter
        if let Some(p) = &plugin_opt {
            sr.register_remote_plugin(p.clone());
        }
        // 注意：不在此处调用 refresh_candidates；
        // 调用方应在批量注册完成后显式调用 refresh()。
    }

    async fn unregister(&self, adapters: &RegisteredAdapters) {
        let sr = self.session_router.clone();
        let cm = self.config_manager.clone();
        let plugin_id = adapters.plugin_id.clone();
        let ds_ids: Vec<String> = adapters
            .data_sources
            .iter()
            .map(|ds| ds.component_id.clone())
            .collect();
        let ex_ids: Vec<String> = adapters
            .executors
            .iter()
            .map(|ex| ex.component_id.clone())
            .collect();
        let config_ids: Vec<String> = adapters
            .configurables
            .iter()
            .map(|c| c.component_id.clone())
            .collect();

        // 解注册 Plugin（先于 DataSource，避免竞态）
        sr.unregister_plugin(&plugin_id);
        // 解注册 DataSource
        for id in &ds_ids {
            sr.unregister_data_source(id).await;
        }
        // 解注册 Executor
        for id in &ex_ids {
            sr.unregister_executor(id);
        }
        // 解注册 Configurable
        for id in &config_ids {
            cm.unregister(id);
        }
    }

    fn register_builtin(&self, collected: &CollectedBuiltins) -> BuiltinPipelineEntries {
        // -- 执行器 --
        for (configurable, executor) in &collected.executors {
            self.config_manager.register(configurable.clone());
            self.session_router.register_executor(executor.clone());
        }

        // -- 数据源（仅注册 Configurable，trait 对象返回给调用方加入管道） --
        for (configurable, _source) in &collected.data_sources {
            self.config_manager.register(configurable.clone());
        }

        // -- 关键词优化器（仅注册 Configurable） --
        for (configurable, _optimizer) in &collected.keyword_optimizers {
            self.config_manager.register(configurable.clone());
        }

        // -- 搜索引擎 --
        for (configurable, engine) in &collected.search_engines {
            self.config_manager.register(configurable.clone());
            self.session_router.register_search_engine(engine.clone());
        }

        // -- 分数增强器 --
        for (configurable, booster) in &collected.score_boosters {
            self.config_manager.register(configurable.clone());
            self.session_router.register_score_booster(booster.clone());
        }

        // -- Plugins --
        for (configurable, plugin) in &collected.plugins {
            self.config_manager.register(configurable.clone());
            self.session_router
                .plugin_service()
                .register(plugin.clone());
        }

        // -- 核心配置组件 --
        for configurable in &collected.core_components {
            self.config_manager.register(configurable.clone());
        }

        // 提取 DataSource 和 KeywordOptimizer 列表供管道构建
        let sources: Vec<_> = collected
            .data_sources
            .iter()
            .map(|(_, ds)| ds.clone())
            .collect();
        let optimizers: Vec<_> = collected
            .keyword_optimizers
            .iter()
            .map(|(_, ko)| ko.clone())
            .collect();

        (sources, optimizers)
    }

    async fn refresh(&self) {
        self.session_router.refresh_candidates().await;
    }
}
