//! Helper functions for constructing mock PluginHandle instances.

use std::sync::Arc;

use parking_lot::RwLock;

use crate::host::{PluginHandle, PluginSdkConfig};
use crate::mock::stubs::*;
use crate::platform::capabilities::PlatformCapabilities;
use crate::services::icon::icon_cache::IconCacheService;
use crate::services::resource::AppResourceService;
use crate::services::timer::TokioTimerManager;
use std::collections::HashSet;

/// 一站式为所有依赖注入 stub 实现并构造 PluginHandle。
/// 所有方法默认返回 Ok(Default::default()) 或空集合。
/// 用于插件的单元测试场景。
pub fn mock_plugin_handle() -> Arc<PluginHandle> {
    let icon_cache = IconCacheService::new("mock_cache".to_string());
    icon_cache.init();
    let storage: Arc<dyn crate::services::storage::storage_service::StorageService> =
        Arc::new(StubStorageService);

    Arc::new(PluginHandle::new(
        "__mock__".to_string(),
        PluginSdkConfig::default(),
        PlatformCapabilities::new(HashSet::new()),
        Arc::new(StubIconExtractor),
        Arc::new(icon_cache),
        Arc::new(StubShellExecutor::default()),
        Arc::new(StubWindowManager),
        Arc::new(StubPathResolver),
        Arc::new(StubAppEnumerator),
        Arc::new(StubAppLauncher),
        Arc::new(StubLnkResolver),
        Arc::new(StubResourceLoader),
        Arc::new(StubParameterResolver),
        Arc::new(TokioTimerManager::new()),
        Arc::new(AppResourceService::new("mock_icons".to_string())),
        Arc::new(RwLock::new(storage)),
        Arc::new(StubHotkeyManager),
        Arc::new(StubInstallationMonitor),
        Arc::new(StubFocusMonitor),
    ))
}
