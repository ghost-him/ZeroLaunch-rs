//! AdapterRegistrar trait 和默认实现。
//!
//! 定义第三方插件适配器的注册/解注册接口，将 PluginManager 与
//! ConfigManager / SessionRouter 解耦。同时提供内置组件注册函数。

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use zerolaunch_plugin_host::manager::RegisteredAdapters;

use crate::core::config::ConfigManager;
use crate::plugin_system::SessionRouter;

use super::builtin_registry::CollectedBuiltins;
use super::types::{DataSource, KeywordOptimizer};

// ── AdapterRegistrar trait ──────────────────────────────────────────

/// 第三方插件适配器的注册/解注册接口。
///
/// 由调用方实现并注入 PluginManager，使 PluginManager 无需直接依赖
/// ConfigManager / SessionRouter。注册逻辑集中在一处，正常加载与
/// 崩溃恢复复用同一实现。
///
/// 返回的 future 为 `'static`：实现内部 clone 所需数据，
/// 调用方无需关心借用的生命周期。
pub trait AdapterRegistrar: Send + Sync {
    /// 注册适配器到各子系统（ConfigManager + SessionRouter）。
    fn register(
        &self,
        adapters: &RegisteredAdapters,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

    /// 从各子系统解注册适配器。
    fn unregister(
        &self,
        adapters: &RegisteredAdapters,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>;
}

// ── DefaultAdapterRegistrar ─────────────────────────────────────────

/// AdapterRegistrar 的默认实现，持有 ConfigManager / SessionRouter 引用。
///
/// 注册行为包含 `sr.refresh_candidates()`，使第三方插件的注册行为完整：
/// 每次注册后自动重建候选项缓存。
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

impl AdapterRegistrar for DefaultAdapterRegistrar {
    fn register(
        &self,
        adapters: &RegisteredAdapters,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>> {
        let cm = self.config_manager.clone();
        let sr = self.session_router.clone();
        let configs = adapters.configurables.clone();
        let sources = adapters.data_sources.clone();
        let executors_list = adapters.executors.clone();
        let plugin_opt = adapters.plugin.clone();

        Box::pin(async move {
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
            // 刷新候选项缓存（第三方插件场景需要）
            sr.refresh_candidates().await;
        })
    }

    fn unregister(
        &self,
        adapters: &RegisteredAdapters,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>> {
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

        Box::pin(async move {
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
        })
    }
}

// ── 内置组件注册 ────────────────────────────────────────────────────

/// 注册内置组件后返回的管道入口：DataSources + KeywordOptimizers。
pub type BuiltinPipelineEntries = (Vec<Arc<dyn DataSource>>, Vec<Arc<dyn KeywordOptimizer>>);

/// 将已收集的内置组件注册到 ConfigManager / SessionRouter。
///
/// 注册规则：
/// - Configurable → ConfigManager::register（所有组件都要）
/// - ActionExecutor → SessionRouter::register_executor
/// - DataSource/KeywordOptimizer → 仅注册 Configurable 部分，trait 对象由调用方管道管理
/// - SearchEngine → SessionRouter::register_search_engine
/// - ScoreBooster → SessionRouter::register_score_booster
/// - Plugin → SessionRouter::plugin_service().register
/// - CoreComponent → 仅 ConfigManager::register
pub fn register_builtin_collected(
    collected: &CollectedBuiltins,
    config_manager: &Arc<ConfigManager>,
    session_router: &Arc<SessionRouter>,
) -> BuiltinPipelineEntries {
    // -- 执行器 --
    for (configurable, executor) in &collected.executors {
        config_manager.register(configurable.clone());
        session_router.register_executor(executor.clone());
    }

    // -- 数据源（仅注册 Configurable，trait 对象返回给调用方加入管道） --
    for (configurable, _source) in &collected.data_sources {
        config_manager.register(configurable.clone());
    }

    // -- 关键词优化器（仅注册 Configurable） --
    for (configurable, _optimizer) in &collected.keyword_optimizers {
        config_manager.register(configurable.clone());
    }

    // -- 搜索引擎 --
    for (configurable, engine) in &collected.search_engines {
        config_manager.register(configurable.clone());
        session_router.register_search_engine(engine.clone());
    }

    // -- 分数增强器 --
    for (configurable, booster) in &collected.score_boosters {
        config_manager.register(configurable.clone());
        session_router.register_score_booster(booster.clone());
    }

    // -- Plugins --
    for (configurable, plugin) in &collected.plugins {
        config_manager.register(configurable.clone());
        session_router.plugin_service().register(plugin.clone());
    }

    // -- 核心配置组件 --
    for configurable in &collected.core_components {
        config_manager.register(configurable.clone());
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
