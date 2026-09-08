//! Helper functions for constructing mock PluginHandle / PluginHost / PlatformServices instances.

use std::collections::HashMap;
use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use parking_lot::RwLock;

use crate::host::{
    build_resource_path, CacheLevel, HostApiError, OpenTarget, PluginHandle, PluginHost,
    PluginSdkConfig,
};
use crate::mock::stubs::*;
use crate::platform::{PlatformCapabilities, PlatformServices};
use crate::services::app::AppInfo;
use crate::services::focus_monitor::FocusCallback;
use crate::services::hotkey::types::{HotkeyCallback, HotkeyEventFilter};
use crate::services::icon::icon_cache::IconCacheService;
use crate::services::installation_monitor::types::InstallationCallback;
use crate::services::model::{
    ModelChatRequest, ModelChatResponse, ModelEmbeddingRequest, ModelEmbeddingResponse, ModelError,
    ModelInfo, ModelService, ModelSimilarityRequest, ModelSimilarityResponse,
};
use crate::services::parameter::resolver::ParameterResolver;
use crate::services::parameter::types::ParameterSnapshot;
use crate::services::path::path_resolver::KnownPath;
use crate::services::resource::AppResourceService;
use crate::services::storage::storage_service::StorageService;
use crate::services::theme::Theme;
use crate::services::timer::{TimerCallback, TimerId, TimerManager, TimerMode, TokioTimerManager};
use crate::services::IconRequest;

/// 构造全桩 PlatformServices（平台端口）：能力集为空，平台服务均为 Stub 实现。
/// 用于测试构造 HostApi（镜像真实平台工厂的组件清单），不触达真实平台能力。
pub fn mock_platform_services() -> PlatformServices {
    PlatformServices {
        capabilities: PlatformCapabilities::new(HashSet::new()),
        icon_extractor: Arc::new(StubIconExtractor),
        shell_executor: Arc::new(StubShellExecutor::default()),
        window_manager: Arc::new(StubWindowManager),
        window_positioner: Arc::new(StubWindowPositioner),
        path_resolver: Arc::new(StubPathResolver),
        app_enumerator: Arc::new(StubAppEnumerator),
        app_launcher: Arc::new(StubAppLauncher),
        lnk_resolver: Arc::new(StubLnkResolver),
        resource_loader: Arc::new(StubResourceLoader),
        clipboard_provider: Arc::new(StubSystemParameterProvider),
        window_handle_provider: Arc::new(StubSystemParameterProvider),
        selection_provider: Arc::new(StubSystemParameterProvider),
        autostart_manager: Arc::new(StubAutoStartManager),
        hotkey_manager: Arc::new(StubHotkeyManager),
        installation_monitor: Arc::new(StubInstallationMonitor),
        focus_monitor: Arc::new(StubFocusMonitor),
        clipboard_manager: Arc::new(StubClipboardManager),
        theme_provider: Arc::new(StubThemeProvider),
    }
}

/// 桩宿主：实现 PluginHost 契约，供 mock_plugin_handle() 构造插件句柄。
/// 平台面委托全桩 PlatformServices，宿主面（缓存/存储/定时器/模型等）为轻量实现，
/// 行为与旧版 mock PluginHandle 一致：各操作返回合理默认值，不触达真实平台。
pub struct MockPluginHost {
    /// 平台面实现集合（全桩，能力集为空）。
    platform: PlatformServices,
    /// 真实图标缓存（跳过 init，避免测试触碰文件系统）
    icon_cache: IconCacheService,
    /// 存储服务（RwLock 提供内部可变性：测试可在运行期替换后端实现）。
    storage: Arc<RwLock<Arc<dyn StorageService>>>,
    /// 定时器管理器（tokio 实现，测试内真实调度）。
    timer: TokioTimerManager,
    /// 参数解析器（桩，返回空串与 0 计数）。
    parameter_resolver: StubParameterResolver,
    /// 应用资源服务（桩，图标/资源查询返回 None）。
    app_resource: AppResourceService,
    /// 模型服务（桩，chat/embedding 返回 NotSupported）。
    model_service: Arc<dyn ModelService>,
    /// 主题模式（RwLock 内部可变性：测试可运行期改写以驱动 get_theme 分支）。
    theme_mode: RwLock<String>,
}

impl MockPluginHost {
    /// 以默认桩组件构造桩宿主。
    pub fn new() -> Self {
        let storage: Arc<dyn StorageService> = Arc::new(StubStorageService);
        Self {
            platform: mock_platform_services(),
            icon_cache: IconCacheService::new("mock_cache".to_string()),
            storage: Arc::new(RwLock::new(storage)),
            timer: TokioTimerManager::new(),
            parameter_resolver: StubParameterResolver,
            app_resource: AppResourceService::new("mock_icons".to_string()),
            model_service: Arc::new(StubModelService),
            theme_mode: RwLock::new("light".to_string()),
        }
    }
}

impl Default for MockPluginHost {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PluginHost for MockPluginHost {
    async fn get_icon(
        &self,
        request: &IconRequest,
        level: CacheLevel,
    ) -> Result<Vec<u8>, HostApiError> {
        self.platform
            .icon_extractor
            .get_icon(&self.icon_cache, request, level)
            .await
    }

    async fn get_icon_or_default(&self, request: &IconRequest, level: CacheLevel) -> Vec<u8> {
        match self
            .platform
            .icon_extractor
            .get_icon(&self.icon_cache, request, level)
            .await
        {
            Ok(data) if !data.is_empty() => data,
            _ => {
                self.platform
                    .icon_extractor
                    .load_default_icon(request)
                    .await
            }
        }
    }

    async fn get_icon_and_update_cache(
        &self,
        request: &IconRequest,
        level: CacheLevel,
    ) -> Result<Vec<u8>, HostApiError> {
        self.platform
            .icon_extractor
            .get_icon_and_update_cache(&self.icon_cache, request, level)
            .await
    }

    async fn override_icon_cache(
        &self,
        _original_request: &IconRequest,
        _custom_icon_path: &str,
    ) -> Result<(), HostApiError> {
        Ok(())
    }

    async fn shell_open(&self, target: OpenTarget) -> Result<(), HostApiError> {
        self.platform.shell_executor.shell_open(&target).await
    }

    async fn shell_open_folder(&self, path: &str) -> Result<(), HostApiError> {
        self.platform.shell_executor.shell_open_folder(path).await
    }

    async fn shell_execute_elevation(&self, path: &str) -> Result<(), HostApiError> {
        self.platform
            .shell_executor
            .shell_execute_elevation(path)
            .await
    }

    async fn shell_execute_command(&self, command: &str) -> Result<(), HostApiError> {
        self.platform
            .shell_executor
            .shell_execute_command(command)
            .await
    }

    async fn activate_window_by_process(&self, process_name: &str) -> Result<bool, HostApiError> {
        self.platform
            .window_manager
            .activate_window_by_process(process_name)
            .await
    }

    async fn activate_window_by_title(&self, title: &str) -> Result<bool, HostApiError> {
        self.platform
            .window_manager
            .activate_window_by_title(title)
            .await
    }

    async fn activate_window_by_pid(&self, pid: u32) -> Result<bool, HostApiError> {
        self.platform
            .window_manager
            .activate_window_by_pid(pid)
            .await
    }

    fn resolve_path(&self, path: KnownPath) -> Result<String, HostApiError> {
        self.platform.path_resolver.resolve_path(path)
    }

    fn set_clipboard_text(&self, text: &str) -> Result<(), HostApiError> {
        self.platform.clipboard_manager.set_text(text)
    }

    async fn enumerate_apps(&self) -> Vec<AppInfo> {
        self.platform.app_enumerator.enumerate_apps().await
    }

    async fn launch_app(&self, app_id: &str, args: Option<&[String]>) -> Result<u32, HostApiError> {
        self.platform.app_launcher.launch_app(app_id, args).await
    }

    fn get_app_icon_path(&self, name: &str) -> Option<String> {
        self.app_resource.get_icon_path(name)
    }

    fn resolve_lnk_target(&self, lnk_path: &str) -> Option<String> {
        self.platform.lnk_resolver.resolve_lnk_target(lnk_path)
    }

    fn parse_localized_names_from_dir(&self, dir_path: &Path) -> HashMap<String, String> {
        self.platform
            .resource_loader
            .parse_localized_names_from_dir(dir_path)
    }

    fn get_theme(&self) -> Result<Theme, HostApiError> {
        match self.theme_mode.read().as_str() {
            "light" => Ok(Theme::Light),
            "dark" => Ok(Theme::Dark),
            _ => self.platform.theme_provider.current_system_theme(),
        }
    }

    fn get_system_theme(&self) -> Result<Theme, HostApiError> {
        self.platform.theme_provider.current_system_theme()
    }

    fn model_list(&self) -> Vec<ModelInfo> {
        self.model_service.list_models()
    }

    async fn model_chat(&self, req: ModelChatRequest) -> Result<ModelChatResponse, ModelError> {
        self.model_service.chat(req).await
    }

    async fn model_embedding(
        &self,
        req: ModelEmbeddingRequest,
    ) -> Result<ModelEmbeddingResponse, ModelError> {
        self.model_service.embedding(req).await
    }

    async fn model_similarity(
        &self,
        req: ModelSimilarityRequest,
    ) -> Result<ModelSimilarityResponse, ModelError> {
        self.model_service.similarity(req).await
    }

    async fn resolve_parameters(
        &self,
        template: &str,
        user_args: &[String],
        snapshot: &ParameterSnapshot,
    ) -> Result<String, HostApiError> {
        self.parameter_resolver
            .resolve(template, user_args, snapshot)
            .await
            .map_err(|e| HostApiError::ParameterResolutionFailed {
                reason: e.to_string(),
            })
    }

    fn count_user_parameters(&self, template: &str) -> usize {
        self.parameter_resolver.count_user_parameters(template)
    }

    fn has_system_parameters(&self, template: &str) -> bool {
        self.parameter_resolver.has_system_parameters(template)
    }

    async fn set_timeout(
        &self,
        delay: Duration,
        callback: TimerCallback,
    ) -> Result<TimerId, HostApiError> {
        self.timer
            .set_timer(delay, TimerMode::OneShot, callback)
            .await
    }

    async fn set_interval(
        &self,
        interval: Duration,
        callback: TimerCallback,
    ) -> Result<TimerId, HostApiError> {
        self.timer
            .set_timer(interval, TimerMode::Interval, callback)
            .await
    }

    async fn cancel_timer(&self, id: TimerId) -> Result<(), HostApiError> {
        self.timer.cancel_timer(id).await
    }

    async fn cancel_all_timers(&self) -> Result<(), HostApiError> {
        self.timer.cancel_all().await
    }

    async fn resource_upload(
        &self,
        plugin_id: &str,
        resource_id: &str,
        _file_path: &str,
        _max_size: Option<u64>,
    ) -> Result<String, HostApiError> {
        let storage_path = build_resource_path(plugin_id, Some(resource_id))?;
        let storage = self.storage.read().clone();
        storage.upload(&storage_path, &[]).await.map_err(|e| {
            HostApiError::StorageOperationFailed {
                file: storage_path,
                reason: e.to_string(),
            }
        })?;
        Ok(resource_id.to_string())
    }

    async fn resource_put(
        &self,
        plugin_id: &str,
        resource_id: &str,
        data: &[u8],
    ) -> Result<(), HostApiError> {
        let storage_path = build_resource_path(plugin_id, Some(resource_id))?;
        let storage = self.storage.read().clone();
        storage.upload(&storage_path, data).await.map_err(|e| {
            HostApiError::StorageOperationFailed {
                file: storage_path,
                reason: e.to_string(),
            }
        })
    }

    async fn resource_get(
        &self,
        plugin_id: &str,
        resource_id: &str,
    ) -> Result<Vec<u8>, HostApiError> {
        let path = build_resource_path(plugin_id, Some(resource_id))?;
        let storage = self.storage.read().clone();
        storage
            .download(&path)
            .await
            .map_err(|e| HostApiError::StorageOperationFailed {
                file: path.clone(),
                reason: e.to_string(),
            })?
            .ok_or_else(|| HostApiError::ResourceNotFound {
                id: resource_id.to_string(),
            })
    }

    async fn resource_delete(
        &self,
        plugin_id: &str,
        resource_id: &str,
    ) -> Result<(), HostApiError> {
        let path = build_resource_path(plugin_id, Some(resource_id))?;
        let storage = self.storage.read().clone();
        storage
            .delete(&path)
            .await
            .map_err(|e| HostApiError::StorageOperationFailed {
                file: path,
                reason: e.to_string(),
            })
    }

    async fn resource_list(&self, plugin_id: &str) -> Result<Vec<String>, HostApiError> {
        let prefix = build_resource_path(plugin_id, None)?;
        let storage = self.storage.read().clone();
        storage
            .list(&prefix)
            .await
            .map_err(|e| HostApiError::StorageOperationFailed {
                file: prefix,
                reason: e.to_string(),
            })
    }

    async fn cache_put(
        &self,
        _plugin_id: &str,
        _domain: &str,
        _key: &str,
        _data: &[u8],
    ) -> Result<(), HostApiError> {
        Ok(())
    }

    async fn cache_get(
        &self,
        _plugin_id: &str,
        _domain: &str,
        _key: &str,
    ) -> Result<Option<Vec<u8>>, HostApiError> {
        Ok(None)
    }

    async fn cache_delete(
        &self,
        _plugin_id: &str,
        _domain: &str,
        _key: &str,
    ) -> Result<(), HostApiError> {
        Ok(())
    }

    async fn cache_cleanup(
        &self,
        _plugin_id: &str,
        _domain: &str,
        _max_entries: usize,
    ) -> Result<(), HostApiError> {
        Ok(())
    }

    fn register_hotkey_callback(
        &self,
        plugin_id: &str,
        id: &str,
        filter: HotkeyEventFilter,
        callback: HotkeyCallback,
    ) {
        let prefixed = format!("{}:{}", plugin_id, id);
        self.platform
            .hotkey_manager
            .register_callback(&prefixed, filter, callback);
    }

    fn unregister_hotkey_callback(&self, plugin_id: &str, id: &str) {
        let prefixed = format!("{}:{}", plugin_id, id);
        self.platform.hotkey_manager.unregister_callback(&prefixed);
    }

    fn register_installation_callback(
        &self,
        plugin_id: &str,
        id: &str,
        callback: InstallationCallback,
    ) {
        let prefixed = format!("{}:{}", plugin_id, id);
        self.platform
            .installation_monitor
            .register_callback(&prefixed, callback);
    }

    fn unregister_installation_callback(&self, plugin_id: &str, id: &str) {
        let prefixed = format!("{}:{}", plugin_id, id);
        self.platform
            .installation_monitor
            .unregister_callback(&prefixed);
    }

    fn register_focus_callback(&self, plugin_id: &str, id: &str, callback: FocusCallback) {
        let prefixed = format!("{}:{}", plugin_id, id);
        self.platform
            .focus_monitor
            .register_callback(&prefixed, callback);
    }

    fn unregister_focus_callback(&self, plugin_id: &str, id: &str) {
        let prefixed = format!("{}:{}", plugin_id, id);
        self.platform.focus_monitor.unregister_callback(&prefixed);
    }
}

/// 一站式为所有依赖注入桩实现并构造 PluginHandle（插件单元测试场景）。
/// 宿主契约为 MockPluginHost；各操作返回合理默认值或空集合。
pub fn mock_plugin_handle() -> Arc<PluginHandle> {
    Arc::new(PluginHandle::new(
        "__mock__".to_string(),
        PluginSdkConfig::default(),
        PlatformCapabilities::new(HashSet::new()),
        Arc::new(MockPluginHost::new()),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::theme::ThemeProvider;

    /// 固定返回深色主题的主题提供器桩，用于验证 get_theme 的 system 分支确实转发平台面。
    struct DarkThemeProvider;

    impl ThemeProvider for DarkThemeProvider {
        fn current_system_theme(&self) -> Result<Theme, HostApiError> {
            Ok(Theme::Dark)
        }
    }

    /// 契约护栏：shell_open 应转发到 platform.shell_executor（记录桩可见），
    /// 而非在 mock 内硬编码返回成功——防止只改 HostApi 忘改 mock 时测试静默失真。
    #[tokio::test]
    async fn shell_open_forwards_to_platform_executor() {
        let recorder = Arc::new(StubShellExecutor::default());
        let mut host = MockPluginHost::new();
        host.platform.shell_executor = recorder.clone();
        host.shell_open(OpenTarget::Url("https://example.com".into()))
            .await
            .unwrap();
        assert_eq!(recorder.opens.lock().len(), 1);
    }

    /// 契约护栏：get_theme 的 system 模式应转发 platform.theme_provider 查询系统主题，
    /// 而非在 mock 内硬编码 light 分支（platform 面已换为固定 Dark 桩以观察转发）。
    #[test]
    fn get_theme_system_mode_forwards_to_platform_provider() {
        let mut host = MockPluginHost::new();
        *host.theme_mode.write() = "system".to_string();
        host.platform.theme_provider = Arc::new(DarkThemeProvider);
        assert_eq!(host.get_theme().unwrap(), Theme::Dark);
    }
}
