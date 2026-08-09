//! 内置组件自动发现注册系统。
//!
//! 使用 `inventory` crate 实现编译期组件收集。每个内置组件在其源文件底部通过
//! `inventory::submit!` 提交工厂函数，本模块在启动时遍历所有已提交的条目并统一注册。
//!
//! 插件作者只需在 `plugin/<category>/` 下加 .rs 文件并添加 `inventory::submit!` 块，
//! 无需修改 `lib.rs`。

use crate::sdk::HostApi;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use zerolaunch_plugin_api::config::Configurable;
use zerolaunch_plugin_api::host::PluginHandle;
use zerolaunch_plugin_api::{
    ActionExecutor, DataSource, KeywordInjector, KeywordOptimizer, Plugin, ScoreBooster,
    SearchEngine,
};

// ============================================================================
// Entry 类型定义 — 每种组件类别一个
// ============================================================================

pub type ExecutorFactory =
    fn(&InventoryContext) -> (Arc<dyn Configurable>, Arc<dyn ActionExecutor>);
pub type DataSourceFactory = fn(&InventoryContext) -> (Arc<dyn Configurable>, Arc<dyn DataSource>);
pub type KeywordOptimizerFactory = fn() -> (Arc<dyn Configurable>, Arc<dyn KeywordOptimizer>);
pub type SearchEngineFactory = fn() -> (Arc<dyn Configurable>, Arc<dyn SearchEngine>);
pub type ScoreBoosterFactory = fn() -> (Arc<dyn Configurable>, Arc<dyn ScoreBooster>);
pub type KeywordInjectorFactory =
    fn(&InventoryContext) -> (Arc<dyn Configurable>, Arc<dyn KeywordInjector>);
pub type PluginFactory = fn() -> (Arc<dyn Configurable>, Arc<dyn Plugin>);
/// 纯配置组件工厂（仅实现 Configurable，不附带其他 trait）。
pub type ConfigComponentFactory = fn(&InventoryContext) -> Arc<dyn Configurable>;

/// inventory 收集结果：所有内置组件的已构造 trait 对象。
///
/// 该 struct 由 `collect_all_builtin_entries()` 返回，
/// 调用方负责将各部分注册到 ConfigManager / SessionDispatcher。
pub struct CollectedBuiltins {
    pub executors: Vec<(Arc<dyn Configurable>, Arc<dyn ActionExecutor>)>,
    pub data_sources: Vec<(Arc<dyn Configurable>, Arc<dyn DataSource>)>,
    pub keyword_optimizers: Vec<(Arc<dyn Configurable>, Arc<dyn KeywordOptimizer>)>,
    pub keyword_injectors: Vec<(Arc<dyn Configurable>, Arc<dyn KeywordInjector>)>,
    pub search_engines: Vec<(Arc<dyn Configurable>, Arc<dyn SearchEngine>)>,
    pub score_boosters: Vec<(Arc<dyn Configurable>, Arc<dyn ScoreBooster>)>,
    pub plugins: Vec<(Arc<dyn Configurable>, Arc<dyn Plugin>)>,
    pub config_components: Vec<Arc<dyn Configurable>>,
}

impl CollectedBuiltins {
    /// 返回所有类别中的 Configurable 引用（按注册顺序，供 async 注册循环使用）。
    pub fn configurables(&self) -> Vec<Arc<dyn Configurable>> {
        let mut out = Vec::with_capacity(64);
        for (c, _) in &self.executors {
            out.push(c.clone());
        }
        for (c, _) in &self.data_sources {
            out.push(c.clone());
        }
        for (c, _) in &self.keyword_optimizers {
            out.push(c.clone());
        }
        for (c, _) in &self.keyword_injectors {
            out.push(c.clone());
        }
        for (c, _) in &self.search_engines {
            out.push(c.clone());
        }
        for (c, _) in &self.score_boosters {
            out.push(c.clone());
        }
        for (c, _) in &self.plugins {
            out.push(c.clone());
        }
        for c in &self.config_components {
            out.push(c.clone());
        }
        out
    }

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
        for (c, _) in &self.keyword_injectors {
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
        for c in &self.config_components {
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

/// 关键词注入器条目。
pub struct KeywordInjectorEntry {
    pub component_id: &'static str,
    pub priority: u32,
    pub factory: KeywordInjectorFactory,
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

/// 纯配置组件条目（仅实现 Configurable，不附带其他 trait）。
pub struct ConfigEntry {
    pub component_id: &'static str,
    pub priority: u32,
    pub factory: ConfigComponentFactory,
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
::inventory::collect!(KeywordInjectorEntry);
::inventory::collect!(ConfigEntry);

// ============================================================================
// InventoryContext — 组件工厂的服务定位器
// ============================================================================

/// 提供给组件工厂的上下文，负责懒创建和缓存 `PluginHandle`。
pub struct InventoryContext {
    host_api: Arc<HostApi>,
    session_dispatcher: Arc<super::SessionDispatcher>,
    handle_cache: RwLock<HashMap<&'static str, Arc<PluginHandle>>>,
}

impl InventoryContext {
    pub fn new(host_api: Arc<HostApi>, session_dispatcher: Arc<super::SessionDispatcher>) -> Self {
        Self {
            host_api,
            session_dispatcher,
            handle_cache: RwLock::new(HashMap::new()),
        }
    }

    pub fn session_dispatcher(&self) -> &Arc<super::SessionDispatcher> {
        &self.session_dispatcher
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
/// 调用方负责将返回的 `CollectedBuiltins` 各部分注册到 ConfigManager / SessionDispatcher。
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

    // -- 关键词注入器 --
    let mut inj_entries: Vec<&KeywordInjectorEntry> =
        ::inventory::iter::<KeywordInjectorEntry>().collect();
    inj_entries.sort_by_key(|e| e.priority);
    let keyword_injectors: Vec<_> = inj_entries.iter().map(|e| (e.factory)(ctx)).collect();

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

    // -- 纯配置组件 --
    let mut cfg_entries: Vec<&ConfigEntry> = ::inventory::iter::<ConfigEntry>().collect();
    cfg_entries.sort_by_key(|e| e.priority);
    let config_components: Vec<_> = cfg_entries.iter().map(|e| (e.factory)(ctx)).collect();

    CollectedBuiltins {
        executors,
        data_sources,
        keyword_optimizers,
        search_engines,
        score_boosters,
        plugins,
        keyword_injectors,
        config_components,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::ConfigManager;
    use crate::plugin_framework::{PluginRegistry, SessionDispatcher};
    use std::collections::HashSet;
    use zerolaunch_plugin_api::config::Configurable;
    use zerolaunch_plugin_api::mock::*;
    use zerolaunch_plugin_api::services::resource::AppResourceService;
    use zerolaunch_plugin_api::services::storage::storage_service::StorageService;
    use zerolaunch_plugin_api::services::timer::TokioTimerManager;
    use zerolaunch_plugin_api::PlatformCapabilities;

    /// 构建仅含桩组件的 HostApi（测试专用，不触达真实平台能力）。
    /// 镜像 lib.rs::build_windows_host_api_builder 的组件清单。
    fn test_host_api() -> Arc<HostApi> {
        let storage: Arc<dyn StorageService> = Arc::new(StubStorageService);
        let api = HostApi::builder("mock_icons".to_string())
            .capabilities(PlatformCapabilities::new(HashSet::new()))
            .icon_extractor(Arc::new(StubIconExtractor))
            .shell_executor(Arc::new(StubShellExecutor::default()))
            .window_manager(Arc::new(StubWindowManager))
            .path_resolver(Arc::new(StubPathResolver))
            .app_enumerator(Arc::new(StubAppEnumerator))
            .app_launcher(Arc::new(StubAppLauncher))
            .lnk_resolver(Arc::new(StubLnkResolver))
            .resource_loader(Arc::new(StubResourceLoader))
            .parameter_resolver(Arc::new(StubParameterResolver))
            .parameter_providers(
                Arc::new(StubSystemParameterProvider),
                Arc::new(StubSystemParameterProvider),
                Arc::new(StubSystemParameterProvider),
            )
            .autostart_manager(Arc::new(StubAutoStartManager))
            .hotkey_manager(Arc::new(StubHotkeyManager))
            .installation_monitor(Arc::new(StubInstallationMonitor))
            .timer_manager(Arc::new(TokioTimerManager::new()))
            .storage_service(storage)
            .app_resource(Arc::new(AppResourceService::new("mock_icons".to_string())))
            .focus_monitor(Arc::new(StubFocusMonitor))
            .clipboard_manager(Arc::new(StubClipboardManager))
            .notify_callback(|_, _| {})
            .hide_window_callback(|| {})
            .show_window_callback(|| {})
            .is_window_visible_callback(|| false)
            .window_positioner(Arc::new(StubWindowPositioner))
            .set_window_position_callback(|_, _| {})
            .build()
            .expect("构建测试 HostApi 失败");
        Arc::new(api)
    }

    /// 组件注册失败的诊断信息：哪个组件、哪个阶段、底层错误。
    struct RegisterFailure {
        component_id: String,
        stage: &'static str,
        error: String,
    }

    /// 注册失败时的修正顺序建议：从源头修复（数据 ↔ schema 对齐），禁止绕过校验。
    const FIX_ORDER_GUIDANCE: &str = "\
  修正顺序建议（从源头修复，禁止绕过校验）：
  1. 定位：对比组件 settings 数据结构与 setting_schema() 声明，
     新增/改名/删除的字段必须同步声明——校验按 schema 键名与类型执行，
     'unknown setting key' 即声明缺失，类型错误即声明与实际不符。
  2. 声明：在 setting_schema() 补齐字段（含嵌套结构、UI 元数据、required 约束）；
     注意 schema 的 string 类型不接受 null，可空值统一以空串编码。
  3. 数据：检查 get_settings()/get_default_settings() 的默认值是否满足 schema 约束
     （非空、最小长度、枚举、类型等）。
  4. 例外：仅当规则确实无法用 schema 表达时才允许覆写 validate_settings，
     且必须剔除自身字段后委托默认校验、保持对未知键的拒绝；
     严禁直接 return Ok(()) 或无条件吞掉校验错误（会让后续字段回归静默失效）。
  5. 验证：修复后重跑本测试。";

    /// 执行与 ConfigManager::register 等价的注册前检查，捕获精确失败原因（诊断镜像，
    /// 与 manager.rs 的 register 检查顺序一致），随后调用真实 register 并确认注册生效。
    /// 返回：Ok(()) 注册成功；Err 携带组件 ID、失败阶段与底层错误。
    async fn register_and_check(
        config_manager: &ConfigManager,
        component: &Arc<dyn Configurable>,
    ) -> Result<(), RegisterFailure> {
        let id = component.component_id().to_string();

        // 阶段 1：schema 本身必须合法（对应 register 的 settings_contribution 检查）。
        if let Err(e) = component.settings_contribution() {
            return Err(RegisterFailure {
                component_id: id,
                stage: "schema 无效",
                error: e.to_string(),
            });
        }

        // 阶段 2：默认配置必须通过校验（对应 register 的 validate_settings(get_settings()) 检查）。
        let settings = component.get_settings();
        if let Err(e) = component.validate_settings(&settings).await {
            return Err(RegisterFailure {
                component_id: id,
                stage: "默认配置校验未通过",
                error: e.to_string(),
            });
        }

        // 阶段 3：真实注册，确认 registry 接纳（覆盖 apply_settings 等其余失败点）。
        config_manager.register(component.clone()).await;
        if config_manager.get_component_schema(&id).is_none() {
            return Err(RegisterFailure {
                component_id: id,
                stage: "register 未生效",
                error: "注册后组件不在 registry 中（多为 apply_settings 失败或重复组件 ID）"
                    .to_string(),
            });
        }
        Ok(())
    }

    /// 全量注册冒烟：镜像启动注册路径，所有内置组件的 schema 有效且默认配置通过校验。
    /// 失败时一次性列出所有失败组件（ID + 阶段 + 底层错误）并附修正顺序建议，
    /// 引导从源头修复（数据 ↔ schema 对齐），防止通过绕过校验的方式让测试变绿。
    /// 回归：translator 曾因 llm_vendor_options 未在 setting_schema 声明，
    /// 注册期 validate_settings 按键名校验拒绝而注册失败。
    #[test]
    fn all_builtin_components_register_with_valid_defaults() {
        let config_manager =
            ConfigManager::new(std::env::temp_dir().join("zl-builtin-register-test"));
        let session_dispatcher = Arc::new(SessionDispatcher::new(Arc::new(PluginRegistry::new())));
        let ctx = InventoryContext::new(test_host_api(), session_dispatcher);
        let collected = collect_all_builtin_entries(&ctx);

        let mut total = 0usize;
        let mut failures: Vec<RegisterFailure> = Vec::new();
        collected.for_each_configurable(&mut |component: &Arc<dyn Configurable>| {
            total += 1;
            if let Err(failure) =
                tauri::async_runtime::block_on(register_and_check(&config_manager, component))
            {
                failures.push(failure);
            }
        });

        assert!(total > 0, "内置组件收集不应为空");
        if !failures.is_empty() {
            let mut detail = String::new();
            for f in &failures {
                detail.push_str(&format!(
                    "  - [{}] {}：{}\n",
                    f.component_id, f.stage, f.error
                ));
            }
            panic!(
                "共 {} / {} 个内置组件注册失败：\n{}\n{}",
                failures.len(),
                total,
                detail,
                FIX_ORDER_GUIDANCE
            );
        }
    }

    /// 诊断输出契约：注册失败的报错必须包含失败组件 ID、失败阶段与底层错误，
    /// 才能按修正顺序指引定位到具体字段。
    #[test]
    fn register_failure_diagnostics_name_component_and_stage() {
        use crate::core::config::setting_builders::SchemaBuilder;
        use serde_json::json;
        use zerolaunch_plugin_api::config::{ComponentCore, ComponentType, SettingDefinition};

        // 伪组件：schema 只声明 known 字段，默认配置却包含未声明键 → 校验必失败。
        struct BrokenComponent {
            core: ComponentCore,
        }
        impl Configurable for BrokenComponent {
            fn core(&self) -> &ComponentCore {
                &self.core
            }
            fn setting_schema(&self) -> Vec<SettingDefinition> {
                vec![SchemaBuilder::text("known", "已知字段", "描述").build()]
            }
            fn get_settings(&self) -> serde_json::Value {
                json!({ "known": "x", "unknown_key": 1 })
            }
        }

        let component: Arc<dyn Configurable> = Arc::new(BrokenComponent {
            core: ComponentCore::new(
                "broken".to_string(),
                "伪组件".to_string(),
                "诊断测试用".to_string(),
                ComponentType::Plugin,
                0,
            ),
        });
        let config_manager = ConfigManager::new(std::env::temp_dir().join("zl-register-diag-test"));

        let failure =
            tauri::async_runtime::block_on(register_and_check(&config_manager, &component))
                .expect_err("含未声明键的伪组件应注册失败");
        assert_eq!(failure.component_id, "broken", "应指出失败组件 ID");
        assert_eq!(failure.stage, "默认配置校验未通过", "应指出失败阶段");
        assert!(
            failure.error.contains("unknown_key"),
            "应指出具体未知键，实际: {}",
            failure.error
        );
    }
}
