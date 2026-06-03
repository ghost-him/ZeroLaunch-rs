//! 内置组件自动发现注册系统。
//!
//! 使用 `inventory` crate 实现编译期组件收集。每个内置组件在其源文件底部通过
//! `inventory::submit!` 提交工厂函数，本模块在启动时遍历所有已提交的条目并统一注册。
//!
//! 插件作者只需在 `plugin/<category>/` 下加 .rs 文件并添加 `inventory::submit!` 块，
//! 无需修改 `lib.rs`。

use crate::core::config::manager::ConfigManager;
use crate::plugin_system::session_router::SessionRouter;
use crate::plugin_system::types::{
    ActionExecutor, DataSource, KeywordOptimizer, Plugin, ScoreBooster, SearchEngine,
};
use crate::plugin_system::Configurable;
use crate::sdk::host_api::HostApi;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use zerolaunch_plugin_api::host::PluginHandle;

// ============================================================================
// Entry 类型定义 — 每种组件类别一个
// ============================================================================

pub type ExecutorFactory =
    fn(&InventoryContext) -> (Arc<dyn Configurable>, Arc<dyn ActionExecutor>);
pub type DataSourceFactory = fn(&InventoryContext) -> (Arc<dyn Configurable>, Arc<dyn DataSource>);
pub type KeywordOptimizerFactory = fn() -> (Arc<dyn Configurable>, Arc<dyn KeywordOptimizer>);
pub type SearchEngineFactory = fn() -> (Arc<dyn Configurable>, Arc<dyn SearchEngine>);
pub type ScoreBoosterFactory = fn() -> (Arc<dyn Configurable>, Arc<dyn ScoreBooster>);
pub type PluginFactory = fn() -> (Arc<dyn Configurable>, Arc<dyn Plugin>);
pub type CoreComponentFactory = fn(Arc<HostApi>) -> Arc<dyn Configurable>;
type RegistrationResult = (Vec<Arc<dyn DataSource>>, Vec<Arc<dyn KeywordOptimizer>>);

/// 执行器条目。
pub struct ExecutorEntry {
    pub component_id: &'static str,
    pub handle_key: &'static str,
    pub priority: u32,
    pub factory: ExecutorFactory,
}

/// 数据源条目。
pub struct DataSourceEntry {
    pub component_id: &'static str,
    pub handle_key: &'static str,
    pub priority: u32,
    pub factory: DataSourceFactory,
}

/// 关键词优化器条目。
pub struct KeywordOptimizerEntry {
    pub component_id: &'static str,
    pub priority: u32,
    pub factory: KeywordOptimizerFactory,
}

/// 搜索引擎条目。
pub struct SearchEngineEntry {
    pub component_id: &'static str,
    pub priority: u32,
    pub factory: SearchEngineFactory,
}

/// 分数增强器条目。
pub struct ScoreBoosterEntry {
    pub component_id: &'static str,
    pub priority: u32,
    pub factory: ScoreBoosterFactory,
}

/// Plugin 条目 (仅 2 个: calculator + everything)。
pub struct PluginEntry {
    pub component_id: &'static str,
    pub priority: u32,
    pub factory: PluginFactory,
}

/// 核心配置组件条目。
pub struct CoreComponentEntry {
    pub component_id: &'static str,
    pub priority: u32,
    pub factory: CoreComponentFactory,
}

// ============================================================================
// Inventory 收集器 — 每种条目一个 collector
// ============================================================================

::inventory::collect!(ExecutorEntry);
::inventory::collect!(DataSourceEntry);
::inventory::collect!(KeywordOptimizerEntry);
::inventory::collect!(SearchEngineEntry);
::inventory::collect!(ScoreBoosterEntry);
::inventory::collect!(PluginEntry);
::inventory::collect!(CoreComponentEntry);

// ============================================================================
// InventoryContext — 组件工厂的服务定位器
// ============================================================================

/// 提供给组件工厂的上下文，负责懒创建和缓存 `PluginHandle`。
pub struct InventoryContext {
    host_api: Arc<HostApi>,
    handle_cache: RwLock<HashMap<&'static str, Arc<PluginHandle>>>,
}

impl InventoryContext {
    pub fn new(host_api: Arc<HostApi>) -> Self {
        Self {
            host_api,
            handle_cache: RwLock::new(HashMap::new()),
        }
    }

    /// 获取或创建指定 key 的 PluginHandle。相同 key 的组件共享同一个 handle。
    pub fn get_handle(&self, key: &'static str) -> Arc<PluginHandle> {
        {
            let cache = self.handle_cache.read();
            if let Some(handle) = cache.get(key) {
                return handle.clone();
            }
        }
        let handle = self.host_api.register(key, Default::default());
        self.handle_cache.write().insert(key, handle.clone());
        handle
    }

    pub fn host_api(&self) -> &Arc<HostApi> {
        &self.host_api
    }
}

// ============================================================================
// 统一注册编排器
// ============================================================================

/// 收集所有 inventory 条目并注册到 ConfigManager / SessionRouter。
///
/// 返回 DataSource 和 KeywordOptimizer 列表供构建 CandidatePipeline。
pub fn register_all_builtin_components(
    ctx: &InventoryContext,
    config_manager: &Arc<ConfigManager>,
    session_router: &Arc<SessionRouter>,
) -> RegistrationResult {
    // -- 执行器 --
    let mut executors: Vec<&ExecutorEntry> = ::inventory::iter::<ExecutorEntry>().collect();
    executors.sort_by_key(|e| e.priority);
    for entry in executors {
        let (configurable, executor) = (entry.factory)(ctx);
        config_manager.register(configurable);
        session_router.register_executor(executor);
    }

    // -- 数据源 --
    let mut sources: Vec<&DataSourceEntry> = ::inventory::iter::<DataSourceEntry>().collect();
    sources.sort_by_key(|e| e.priority);
    let mut source_list = Vec::new();
    for entry in sources {
        let (configurable, source) = (entry.factory)(ctx);
        config_manager.register(configurable);
        source_list.push(source);
    }

    // -- 关键词优化器 --
    let mut optimizers: Vec<&KeywordOptimizerEntry> =
        ::inventory::iter::<KeywordOptimizerEntry>().collect();
    optimizers.sort_by_key(|e| e.priority);
    let mut optimizer_list = Vec::new();
    for entry in optimizers {
        let (configurable, optimizer) = (entry.factory)();
        config_manager.register(configurable);
        optimizer_list.push(optimizer);
    }

    // -- 搜索引擎 --
    let mut engines: Vec<&SearchEngineEntry> = ::inventory::iter::<SearchEngineEntry>().collect();
    engines.sort_by_key(|e| e.priority);
    for entry in engines {
        let (configurable, engine) = (entry.factory)();
        config_manager.register(configurable);
        session_router.register_search_engine(engine);
    }

    // -- 分数增强器 --
    let mut boosters: Vec<&ScoreBoosterEntry> = ::inventory::iter::<ScoreBoosterEntry>().collect();
    boosters.sort_by_key(|e| e.priority);
    for entry in boosters {
        let (configurable, booster) = (entry.factory)();
        config_manager.register(configurable);
        session_router.register_score_booster(booster);
    }

    // -- Plugins --
    let mut plugins: Vec<&PluginEntry> = ::inventory::iter::<PluginEntry>().collect();
    plugins.sort_by_key(|e| e.priority);
    for entry in plugins {
        let (configurable, plugin) = (entry.factory)();
        config_manager.register(configurable.clone());
        session_router.plugin_service().register(plugin);
    }

    // -- 核心配置组件 --
    let mut core_components: Vec<&CoreComponentEntry> =
        ::inventory::iter::<CoreComponentEntry>().collect();
    core_components.sort_by_key(|e| e.priority);
    for entry in core_components {
        let configurable = (entry.factory)(ctx.host_api().clone());
        config_manager.register(configurable);
    }

    (source_list, optimizer_list)
}
