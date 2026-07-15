use async_trait::async_trait;
use parking_lot::RwLock;
use std::sync::Arc;
use zerolaunch_plugin_api::config::{ComponentCore, ComponentType, ConfigError, Configurable};
use zerolaunch_plugin_api::host::PluginHandle;
use zerolaunch_plugin_api::{CachedCandidateData, DataSource, ExecutionTarget, SearchCandidate};

/// 应用数据源 - 通过 PluginHandle 枚举系统应用（UWP 等）。
/// 不再直接调用 Win32 API，而是委托 PluginHandle::enumerate_apps() 由 SDK 层处理平台差异。
pub struct AppSource {
    core: ComponentCore,
    plugin_handle: Arc<PluginHandle>,
    settings: RwLock<serde_json::Value>,
}

impl AppSource {
    pub fn new(plugin_handle: Arc<PluginHandle>) -> Self {
        AppSource {
            core: ComponentCore::new(
                "app-source".to_string(),
                "应用数据源".to_string(),
                "从开始菜单和已安装应用列表中搜索应用".to_string(),
                ComponentType::DataSource,
                10,
            ),
            plugin_handle,
            settings: RwLock::new(serde_json::Value::Null),
        }
    }
}

#[async_trait]
impl Configurable for AppSource {
    fn core(&self) -> &ComponentCore {
        &self.core
    }

    fn get_settings(&self) -> serde_json::Value {
        self.settings.read().clone()
    }

    fn apply_settings(&self, settings: serde_json::Value) -> Result<(), ConfigError> {
        *self.settings.write() = settings;
        Ok(())
    }
}

#[async_trait]
impl DataSource for AppSource {
    /// 枚举系统应用并转换为搜索候选项。
    /// 委托 PluginHandle::enumerate_apps() 获取应用列表，将 AppInfo 映射为 SearchCandidate。
    async fn fetch_candidates(&self) -> CachedCandidateData {
        let mut result = CachedCandidateData::new();

        let apps = self.plugin_handle.enumerate_apps().await;

        for app_info in apps {
            let candidate = SearchCandidate {
                id: 0,
                name: app_info.display_name,
                icon: app_info.icon,
                target: ExecutionTarget::App(app_info.app_id),
                keywords: Vec::new(),
                bias: 0.0,
                trigger_keywords: Vec::new(),
            };

            result.add_candidate(candidate);
        }

        result
    }
}

use crate::plugin_framework::builtin_registry::{DataSourceEntry, InventoryContext};

pub(crate) fn build_app_source(
    ctx: &InventoryContext,
) -> (Arc<dyn Configurable>, Arc<dyn DataSource>) {
    let handle = ctx.get_handle("app-source");
    let source: Arc<dyn DataSource> = Arc::new(AppSource::new(handle));
    let configurable: Arc<dyn Configurable> = source.clone();
    (configurable, source)
}

::inventory::submit! {
    DataSourceEntry {
        component_id: "app-source",
        handle_key: "app-source",
        priority: 10,
        factory: build_app_source,
    }
}
