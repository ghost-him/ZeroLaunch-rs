//! 内置组件自动发现注册系统。
//!
//! 使用 `inventory` crate 实现编译期组件收集。每个内置组件在其源文件底部通过
//! `inventory::submit!` 提交工厂函数，本模块在启动时遍历所有已提交的条目并统一注册。
//!
//! 插件作者只需在 `plugin/<category>/` 下加 .rs 文件并添加 `inventory::submit!` 块，
//! 无需修改 `lib.rs`。

use crate::core::config::core_registry::CoreComponentEntry;
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
/// inventory 收集结果：所有内置组件的已构造 trait 对象。
///
/// 该 struct 由 `collect_all_builtin_entries()` 返回，
/// 调用方负责将各部分注册到 ConfigManager / SessionRouter。
pub struct CollectedBuiltins {
    pub executors: Vec<(Arc<dyn Configurable>, Arc<dyn ActionExecutor>)>,
    pub data_sources: Vec<(Arc<dyn Configurable>, Arc<dyn DataSource>)>,
    pub keyword_optimizers: Vec<(Arc<dyn Configurable>, Arc<dyn KeywordOptimizer>)>,
    pub search_engines: Vec<(Arc<dyn Configurable>, Arc<dyn SearchEngine>)>,
    pub score_boosters: Vec<(Arc<dyn Configurable>, Arc<dyn ScoreBooster>)>,
    pub plugins: Vec<(Arc<dyn Configurable>, Arc<dyn Plugin>)>,
    pub core_components: Vec<Arc<dyn Configurable>>,
}

impl CollectedBuiltins {
    /// 遍历所有类别中的 Configurable 并调用 `f`。
    pub fn for_each_configurable(&self, mut f: impl FnMut(&Arc<dyn Configurable>)) {
        for (c, _) in &self.executors {
            f(c);
        }
        for (c, _) in &self.data_sources {
            f(c);
        }
        for (c, _) in &self.keyword_optimizers {
            f(c);
        }
        for (c, _) in &self.search_engines {
            f(c);
        }
        for (c, _) in &self.score_boosters {
            f(c);
        }
        for (c, _) in &self.plugins {
            f(c);
        }
        for c in &self.core_components {
            f(c);
        }
    }
}

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

/// Plugin 条目。
pub struct PluginEntry {
    pub component_id: &'static str,
    pub priority: u32,
    pub factory: PluginFactory,
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
// 统一收集 — 纯收集，不做注册
// ============================================================================

/// 收集所有 inventory 条目，调用工厂构造 trait 对象，但不注册到任何管理器。
///
/// 调用方负责将返回的 `CollectedBuiltins` 各部分注册到 ConfigManager / SessionRouter。
/// 这样设计的目的是将「有哪些组件」与「组件注册到哪里」解耦，
/// 让 PluginManager 成为注册编排的唯一权威。
pub fn collect_all_builtin_entries(ctx: &InventoryContext) -> CollectedBuiltins {
    // -- 执行器 --
    let mut exec_entries: Vec<&ExecutorEntry> = ::inventory::iter::<ExecutorEntry>().collect();
    exec_entries.sort_by_key(|e| e.priority);
    let executors: Vec<_> = exec_entries.iter().map(|e| (e.factory)(ctx)).collect();

    // -- 数据源 --
    let mut src_entries: Vec<&DataSourceEntry> = ::inventory::iter::<DataSourceEntry>().collect();
    src_entries.sort_by_key(|e| e.priority);
    let data_sources: Vec<_> = src_entries.iter().map(|e| (e.factory)(ctx)).collect();

    // -- 关键词优化器 --
    let mut opt_entries: Vec<&KeywordOptimizerEntry> =
        ::inventory::iter::<KeywordOptimizerEntry>().collect();
    opt_entries.sort_by_key(|e| e.priority);
    let keyword_optimizers: Vec<_> = opt_entries.iter().map(|e| (e.factory)()).collect();

    // -- 搜索引擎 --
    let mut eng_entries: Vec<&SearchEngineEntry> =
        ::inventory::iter::<SearchEngineEntry>().collect();
    eng_entries.sort_by_key(|e| e.priority);
    let search_engines: Vec<_> = eng_entries.iter().map(|e| (e.factory)()).collect();

    // -- 分数增强器 --
    let mut boo_entries: Vec<&ScoreBoosterEntry> =
        ::inventory::iter::<ScoreBoosterEntry>().collect();
    boo_entries.sort_by_key(|e| e.priority);
    let score_boosters: Vec<_> = boo_entries.iter().map(|e| (e.factory)()).collect();

    // -- Plugins --
    let mut plug_entries: Vec<&PluginEntry> = ::inventory::iter::<PluginEntry>().collect();
    plug_entries.sort_by_key(|e| e.priority);
    let plugins: Vec<_> = plug_entries.iter().map(|e| (e.factory)()).collect();

    // -- 核心配置组件 --
    let mut core_entries: Vec<&CoreComponentEntry> =
        ::inventory::iter::<CoreComponentEntry>().collect();
    core_entries.sort_by_key(|e| e.priority);
    let core_components: Vec<_> = core_entries
        .iter()
        .map(|e| (e.factory)(ctx.host_api().clone()))
        .collect();

    CollectedBuiltins {
        executors,
        data_sources,
        keyword_optimizers,
        search_engines,
        score_boosters,
        plugins,
        core_components,
    }
}
