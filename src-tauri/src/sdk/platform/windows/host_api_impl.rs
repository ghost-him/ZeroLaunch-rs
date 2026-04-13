use crate::sdk::host_api::{
    CacheLevel, HostApi, HostApiError, IconRequest, OpenTarget, PluginHandle, PluginSdkConfig,
};
use crate::sdk::platform::capabilities::PlatformCapabilities;
use async_trait::async_trait;
use dashmap::DashMap;
use parking_lot::RwLock;
use std::sync::Arc;

/// Windows 平台的插件服务句柄。
/// 持有插件 ID 和配置，服务调用时根据配置决定行为。
pub struct WindowsPluginHandle {
    plugin_id: String,
    config: RwLock<PluginSdkConfig>,
    capabilities: PlatformCapabilities,
}

impl WindowsPluginHandle {
    /// 创建 Windows 平台的插件服务句柄。
    /// 参数：plugin_id - 插件唯一标识；config - 插件 SDK 配置。
    /// 返回：初始化后的 WindowsPluginHandle。
    pub fn new(plugin_id: String, config: PluginSdkConfig) -> Self {
        Self {
            plugin_id,
            config: RwLock::new(config),
            capabilities: PlatformCapabilities::windows(),
        }
    }

    /// 获取当前图标缓存等级，None 时返回默认值 Full。
    fn icon_cache_level(&self) -> CacheLevel {
        self.config.read().icon_cache_level.unwrap_or_default()
    }

    /// 获取插件 ID。
    #[allow(dead_code)]
    fn plugin_id(&self) -> &str {
        &self.plugin_id
    }
}

#[async_trait]
impl PluginHandle for WindowsPluginHandle {
    async fn get_icon(&self, _request: IconRequest) -> Result<Vec<u8>, HostApiError> {
        let _level = self.icon_cache_level();
        todo!("迁移 IconManager 后实现")
    }

    async fn get_icon_and_update_cache(
        &self,
        _request: IconRequest,
    ) -> Result<Vec<u8>, HostApiError> {
        let _level = self.icon_cache_level();
        todo!("迁移 IconManager 后实现")
    }

    async fn shell_open(&self, _target: OpenTarget) -> Result<(), HostApiError> {
        todo!("迁移 Shell 服务后实现")
    }

    async fn shell_open_folder(&self, _path: &str) -> Result<(), HostApiError> {
        todo!("迁移 Shell 服务后实现")
    }

    async fn get_default_browser(&self) -> Result<String, HostApiError> {
        todo!("迁移 Shell 服务后实现")
    }

    async fn activate_window_by_process(&self, _process_name: &str) -> Result<bool, HostApiError> {
        todo!("迁移窗口服务后实现")
    }

    async fn activate_window_by_title(&self, _title: &str) -> Result<bool, HostApiError> {
        todo!("迁移窗口服务后实现")
    }

    fn update_config(&self, config: PluginSdkConfig) {
        *self.config.write() = config;
    }

    fn capabilities(&self) -> &PlatformCapabilities {
        &self.capabilities
    }
}

/// Windows 平台的 HostApi 实现。
/// 管理插件注册表，通过 register() 创建 PluginHandle。
pub struct WindowsHostApi {
    handles: DashMap<String, Arc<WindowsPluginHandle>>,
    capabilities: PlatformCapabilities,
}

impl WindowsHostApi {
    /// 创建 Windows 平台的 HostApi 实例。
    /// 参数：无。
    /// 返回：初始化后的 WindowsHostApi，具备 Windows 平台完整能力集。
    pub fn new() -> Self {
        Self {
            handles: DashMap::new(),
            capabilities: PlatformCapabilities::windows(),
        }
    }
}

impl Default for WindowsHostApi {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl HostApi for WindowsHostApi {
    fn register(&self, plugin_id: &str, config: PluginSdkConfig) -> Arc<dyn PluginHandle> {
        let handle = Arc::new(WindowsPluginHandle::new(plugin_id.to_string(), config));
        self.handles.insert(plugin_id.to_string(), handle.clone());
        handle
    }

    async fn update_icon_cache_dir(&self, _new_icon_cache_dir: &str) -> Result<(), HostApiError> {
        todo!("迁移 IconManager 后实现")
    }

    fn capabilities(&self) -> &PlatformCapabilities {
        &self.capabilities
    }
}
