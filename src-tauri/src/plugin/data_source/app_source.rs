use crate::plugin_system::cached_candidate::CachedCandidateData;
use crate::plugin_system::types::{DataSource, ExecutionTarget, SearchCandidate};
use crate::plugin_system::{ComponentType, ConfigError, Configurable};
use async_trait::async_trait;
use parking_lot::RwLock;
use std::sync::Arc;
use zerolaunch_plugin_api::host::PluginHandle;

/// 应用数据源 - 通过 PluginHandle 枚举系统应用（UWP 等）。
/// 不再直接调用 Win32 API，而是委托 PluginHandle::enumerate_apps() 由 SDK 层处理平台差异。
pub struct AppSource {
    plugin_handle: Arc<PluginHandle>,
    settings: RwLock<serde_json::Value>,
}

impl AppSource {
    pub fn new(plugin_handle: Arc<PluginHandle>) -> Self {
        AppSource {
            plugin_handle,
            settings: RwLock::new(serde_json::Value::Null),
        }
    }
}

impl Configurable for AppSource {
    fn component_id(&self) -> &str {
        "app-source"
    }

    fn component_name(&self) -> &str {
        "应用数据源"
    }

    fn component_type(&self) -> ComponentType {
        ComponentType::DataSource
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
