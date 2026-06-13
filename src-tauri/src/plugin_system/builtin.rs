//! BuiltinProvider — 内置组件提供者。
//!
//! 包装 inventory 收集 + 注册编排，将内置组件注册到 ConfigManager / SessionRouter，
//! 并在 PluginManager 中创建对应的 PluginInfo 条目。

use crate::core::config::ConfigManager;
use crate::sdk::HostApi;
use std::sync::Arc;

use super::builtin_registry::{self, CollectedBuiltins, InventoryContext};
use super::plugin_info::{PluginInfo, PluginKind, PluginStatus};
use super::SessionRouter;

/// Return type for builtin component initialization:
/// (data sources for CandidatePipeline, keyword optimizers for CandidatePipeline).
pub type BuiltinInitResult = (
    Vec<Arc<dyn super::types::DataSource>>,
    Vec<Arc<dyn super::types::KeywordOptimizer>>,
);

/// 内置组件提供者。
///
/// 负责：
/// 1. 通过 inventory 收集所有内置组件（委托 `builtin_registry::collect_all_builtin_entries`）
/// 2. 将各部分注册到 ConfigManager / SessionRouter
/// 3. 为每个内置组件创建 PluginInfo 条目并注册到 PluginManager
pub struct BuiltinProvider;

impl BuiltinProvider {
    /// 创建 BuiltinProvider。
    pub fn new() -> Self {
        Self
    }

    /// 初始化所有内置组件。
    ///
    /// 收集 inventory 条目 → 注册到 ConfigManager / SessionRouter → 创建 PluginInfo。
    ///
    /// # 参数
    /// - `host_api`: 用于创建 InventoryContext
    /// - `config_manager`: 用于注册 Configurable
    /// - `session_router`: 用于注册 Executor / SearchEngine / ScoreBooster / Plugin
    /// - `register_info`: 回调，每注册一个组件调用一次，用于创建 PluginInfo
    ///
    /// # 返回
    /// DataSource 和 KeywordOptimizer 列表，供调用方构建 CandidatePipeline。
    /// 注意：DataSource/KeywordOptimizer 不在 SessionRouter 中注册（内置组件直接加入管道），
    /// 仅它们的 Configurable 部分注册到 ConfigManager。
    pub fn init(
        &self,
        host_api: &Arc<HostApi>,
        config_manager: &Arc<ConfigManager>,
        session_router: &Arc<SessionRouter>,
        register_info: &mut dyn FnMut(PluginInfo),
    ) -> BuiltinInitResult {
        let ctx = InventoryContext::new(host_api.clone());
        let collected = builtin_registry::collect_all_builtin_entries(&ctx);

        self.register_collected(&collected, config_manager, session_router, register_info);

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

    /// 将已收集的内置组件注册到各管理器并创建 PluginInfo。
    ///
    /// 注册规则（与旧 `register_all_builtin_components` 保持一致）：
    /// - Configurable → ConfigManager::register（所有组件都要）
    /// - ActionExecutor → SessionRouter::register_executor
    /// - DataSource/KeywordOptimizer → 仅注册 Configurable 部分，trait 对象由调用方管道管理
    /// - SearchEngine → SessionRouter::register_search_engine
    /// - ScoreBooster → SessionRouter::register_score_booster
    /// - Plugin → SessionRouter::plugin_service().register
    /// - CoreComponent → 仅 ConfigManager::register
    fn register_collected(
        &self,
        collected: &CollectedBuiltins,
        config_manager: &Arc<ConfigManager>,
        session_router: &Arc<SessionRouter>,
        register_info: &mut dyn FnMut(PluginInfo),
    ) {
        // -- 执行器 --
        for (configurable, executor) in &collected.executors {
            let enabled = configurable.default_enabled();
            config_manager.register(configurable.clone());
            session_router.register_executor(executor.clone());
            register_info(Self::make_builtin_info(configurable, enabled));
        }

        // -- 数据源（仅注册 Configurable，trait 对象返回给调用方加入管道） --
        for (configurable, _source) in &collected.data_sources {
            let enabled = configurable.default_enabled();
            config_manager.register(configurable.clone());
            register_info(Self::make_builtin_info(configurable, enabled));
        }

        // -- 关键词优化器（仅注册 Configurable） --
        for (configurable, _optimizer) in &collected.keyword_optimizers {
            let enabled = configurable.default_enabled();
            config_manager.register(configurable.clone());
            register_info(Self::make_builtin_info(configurable, enabled));
        }

        // -- 搜索引擎 --
        for (configurable, engine) in &collected.search_engines {
            let enabled = configurable.default_enabled();
            config_manager.register(configurable.clone());
            session_router.register_search_engine(engine.clone());
            register_info(Self::make_builtin_info(configurable, enabled));
        }

        // -- 分数增强器 --
        for (configurable, booster) in &collected.score_boosters {
            let enabled = configurable.default_enabled();
            config_manager.register(configurable.clone());
            session_router.register_score_booster(booster.clone());
            register_info(Self::make_builtin_info(configurable, enabled));
        }

        // -- Plugins --
        for (configurable, plugin) in &collected.plugins {
            let enabled = configurable.default_enabled();
            config_manager.register(configurable.clone());
            session_router.plugin_service().register(plugin.clone());
            register_info(Self::make_builtin_info(configurable, enabled));
        }

        // -- 核心配置组件 --
        for configurable in &collected.core_components {
            let enabled = configurable.default_enabled();
            config_manager.register(configurable.clone());
            register_info(Self::make_builtin_info(configurable, enabled));
        }
    }

    /// 为标准内置组件创建 PluginInfo。
    fn make_builtin_info(
        configurable: &Arc<dyn crate::plugin_system::Configurable>,
        enabled: bool,
    ) -> PluginInfo {
        PluginInfo {
            id: configurable.component_id().to_string(),
            name: configurable.component_name().to_string(),
            kind: PluginKind::Builtin,
            status: PluginStatus::Active,
            version: None,
            description: None,
            author: Some("ZeroLaunch".to_string()),
            component_count: 1,
            enabled,
        }
    }
}

impl Default for BuiltinProvider {
    fn default() -> Self {
        Self::new()
    }
}
