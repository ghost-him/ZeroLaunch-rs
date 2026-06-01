use crate::sdk::app::app_enumerator::AppEnumerator;
use crate::sdk::app::app_launcher::AppLauncher;
use crate::sdk::autostart::AutoStartManager;
use crate::sdk::focus_monitor::FocusMonitor;
use crate::sdk::hotkey::types::HotkeyConfig;
use crate::sdk::hotkey::HotkeyManager;
use crate::sdk::icon::icon_cache::IconCacheService;
use crate::sdk::icon::icon_extractor::IconExtractor;
use crate::sdk::installation_monitor::InstallationMonitor;
use crate::sdk::parameter::provider::SystemParameterProvider;
use crate::sdk::parameter::resolver::ParameterResolver;
use crate::sdk::parameter::types::ParameterSnapshot;
use crate::sdk::path::path_resolver::PathResolver;
use crate::sdk::platform::capabilities::PlatformCapabilities;
use crate::sdk::resource::AppResourceService;
use crate::sdk::shell::lnk_resolver::LnkResolver;
use crate::sdk::shell::resource_loader::ResourceLoader;
use crate::sdk::shell::ShellExecutor;
use crate::sdk::storage::storage_service::StorageService;
use crate::sdk::timer::TimerManager;
use crate::sdk::window::{WindowManager, WindowPosition, WindowPositioner};
use dashmap::DashMap;
use parking_lot::RwLock;

use std::sync::Arc;

// Re-export from plugin-api
pub use zerolaunch_plugin_api::host::{
    CacheLevel, HostApiBuildError, HostApiError, OpenTarget, PluginHandle, PluginSdkConfig,
};

/// 宿主向插件暴露的平台能力注册层。
/// 跨平台 struct，平台组件在构造时注入。
/// 插件必须先调用 register() 获取 PluginHandle，再通过句柄访问服务。
/// 全局管理操作（如更新缓存目录）保留在 HostApi 上，不暴露给插件。
pub struct HostApi {
    handles: DashMap<String, Arc<PluginHandle>>,
    capabilities: PlatformCapabilities,
    /// 共享的图标缓存服务
    icon_cache: Arc<IconCacheService>,
    /// 图标提取器（平台实现）
    icon_extractor: Arc<dyn IconExtractor>,
    /// Shell 执行器（平台实现）
    shell_executor: Arc<dyn ShellExecutor>,
    /// 窗口管理器（平台实现）
    window_manager: Arc<dyn WindowManager>,
    /// 窗口位置计算器（平台实现）
    window_positioner: Arc<dyn WindowPositioner>,
    /// 设置窗口位置回调（宿主级）
    set_window_position_callback: RwLock<Arc<dyn Fn(i32, i32) + Send + Sync + 'static>>,
    /// 路径解析器（平台实现）
    path_resolver: Arc<dyn PathResolver>,
    /// 应用枚举器（平台实现）
    app_enumerator: Arc<dyn AppEnumerator>,
    /// 应用启动器（平台实现）
    app_launcher: Arc<dyn AppLauncher>,
    /// Lnk 快捷方式解析器（平台实现）
    lnk_resolver: Arc<dyn LnkResolver>,
    /// 资源加载器（平台实现）
    resource_loader: Arc<dyn ResourceLoader>,
    /// 参数解析器
    parameter_resolver: Arc<dyn ParameterResolver>,
    /// 剪贴板参数提供者（平台实现）
    clipboard_provider: Arc<dyn SystemParameterProvider>,
    /// 窗口句柄参数提供者（平台实现）
    window_handle_provider: Arc<dyn SystemParameterProvider>,
    /// 选中文本参数提供者（平台实现）
    selection_provider: Arc<dyn SystemParameterProvider>,
    /// 自启动管理器（平台实现）
    autostart_manager: Arc<dyn AutoStartManager>,
    /// 按键管理器（平台实现）
    hotkey_manager: Arc<dyn HotkeyManager>,
    /// 安装监控器（平台实现）
    installation_monitor: Arc<dyn InstallationMonitor>,
    /// 聚焦监控器（平台实现，可选，由 init_search_bar_window 设置）
    focus_monitor: Arc<dyn FocusMonitor>,
    /// 定时器管理器
    timer_manager: Arc<dyn TimerManager>,
    /// 存储服务（可运行时重配置：Local ↔ WebDAV）
    storage: Arc<RwLock<Arc<dyn StorageService>>>,
    /// 应用资源服务
    app_resource: Arc<AppResourceService>,
    /// 通知回调（宿主级）
    notify_callback: RwLock<Arc<dyn Fn(String, String) + Send + Sync + 'static>>,
    /// 隐藏窗口回调（宿主级）
    hide_window_callback: RwLock<Arc<dyn Fn() + Send + Sync + 'static>>,
    /// 显示窗口回调（宿主级）
    show_window_callback: RwLock<Arc<dyn Fn() + Send + Sync + 'static>>,
    /// 查询窗口可见性回调（宿主级）
    is_window_visible_callback: RwLock<Arc<dyn Fn() -> bool + Send + Sync + 'static>>,
}

impl HostApi {
    /// 创建 HostApiBuilder，用于构建 HostApi 实例。
    /// 参数：icon_cache_dir - 图标缓存目录。
    /// 返回：HostApiBuilder 实例。
    pub fn builder(icon_cache_dir: String) -> HostApiBuilder {
        HostApiBuilder::new(icon_cache_dir)
    }

    /// 注册插件并返回绑定了插件身份与配置的服务句柄。
    /// 参数：plugin_id - 插件唯一标识；config - 插件的 SDK 配置。
    /// 返回：绑定该插件配置的服务句柄。
    /// 特性：同一 plugin_id 重复注册将覆盖原有配置。
    pub fn register(&self, plugin_id: &str, config: PluginSdkConfig) -> Arc<PluginHandle> {
        let handle = Arc::new(PluginHandle::new(
            plugin_id.to_string(),
            config,
            self.capabilities.clone(),
            self.icon_extractor.clone(),
            self.icon_cache.clone(),
            self.shell_executor.clone(),
            self.window_manager.clone(),
            self.path_resolver.clone(),
            self.app_enumerator.clone(),
            self.app_launcher.clone(),
            self.lnk_resolver.clone(),
            self.resource_loader.clone(),
            self.parameter_resolver.clone(),
            self.timer_manager.clone(),
            self.app_resource.clone(),
            self.storage.clone(),
            self.hotkey_manager.clone(),
            self.installation_monitor.clone(),
            self.focus_monitor.clone(),
        ));
        self.handles.insert(plugin_id.to_string(), handle.clone());
        handle
    }

    /// 更新图标缓存目录路径（宿主级操作，不暴露给插件）。
    /// 参数：new_icon_cache_dir - 新的图标文件缓存目录路径。
    /// 返回：成功返回 Ok(())，失败返回 HostApiError。
    /// 特性：切换文件缓存的存储位置，同时清空内存缓存以保持一致性。
    pub fn update_icon_cache_dir(&self, new_icon_cache_dir: &str) -> Result<(), HostApiError> {
        self.icon_cache.update_cache_dir(new_icon_cache_dir);
        Ok(())
    }

    /// 查询当前平台支持的能力集合。
    /// 参数：无。
    /// 返回：平台能力的不可变引用。
    pub fn capabilities(&self) -> &PlatformCapabilities {
        &self.capabilities
    }

    // ===== 应用资源服务 =====

    /// 根据名称获取内置图标资源的文件系统路径。
    /// 参数：name - 图标名称（如 "tray_icon", "web_pages" 等）。
    /// 返回：图标路径，未注册则返回 None。
    pub fn get_app_icon_path(&self, name: &str) -> Option<String> {
        self.app_resource.get_icon_path(name)
    }

    // ===== 通知服务 =====

    /// 发送桌面通知。
    /// 参数：title - 通知标题；message - 通知内容。
    pub async fn notify(&self, title: &str, message: &str) {
        self.notify_callback.read()(title.to_string(), message.to_string());
    }

    // ===== 窗口控制 =====

    /// 隐藏搜索栏窗口。
    pub async fn hide_window(&self) {
        self.hide_window_callback.read()();
    }

    /// 显示搜索栏窗口。
    pub async fn show_window(&self) {
        self.show_window_callback.read()();
    }

    /// 查询搜索栏窗口是否可见。
    /// 直接查询窗口真实状态，不依赖缓存变量。
    pub fn is_window_visible(&self) -> bool {
        self.is_window_visible_callback.read()()
    }

    /// 计算窗口最优显示位置。
    /// 委托给平台 WindowPositioner 实现，根据配置和系统状态返回物理像素坐标。
    pub async fn compute_window_position(
        &self,
        request: crate::sdk::window::PositionRequest,
    ) -> Result<WindowPosition, HostApiError> {
        self.window_positioner.compute_position(request).await
    }

    /// 设置搜索栏窗口位置（物理像素坐标）。
    pub fn set_window_position(&self, position: WindowPosition) {
        self.set_window_position_callback.read()(position.x, position.y);
    }

    /// 捕获当前系统参数快照
    ///
    /// 调用时机：唤醒搜索栏时，由宿主调用（非插件调用）。
    /// 通过各 Provider 获取当前时刻的系统参数值，封装为不透明句柄。
    ///
    /// 返回：ParameterSnapshot 不透明句柄
    pub async fn capture_parameter_snapshot(&self) -> ParameterSnapshot {
        let mut snapshot = ParameterSnapshot::empty();

        if let Ok(value) = self.clipboard_provider.get_value().await {
            snapshot.insert("clipboard".to_string(), value);
        }

        if let Ok(value) = self.window_handle_provider.get_value().await {
            snapshot.insert("hwnd".to_string(), value);
        }

        if let Ok(value) = self.selection_provider.get_value().await {
            snapshot.insert("selection".to_string(), value);
        }

        snapshot
    }

    // ===== 自启动服务 =====

    /// 应用自启动设置。根据 enabled 启用或禁用自启动。
    ///
    /// 参数：enabled - 是否启用自启动
    /// 返回：成功返回 Ok(())，失败返回 HostApiError
    ///
    /// 此方法供核心程序调用，根据配置自动启用或禁用自启动。
    pub async fn apply_autostart_setting(&self, enabled: bool) -> Result<(), HostApiError> {
        let task_name = self.autostart_manager.default_task_name();
        if enabled {
            let exe_path = std::env::current_exe()
                .map_err(|e| HostApiError::AutoStartFailed {
                    reason: format!("获取可执行文件路径失败: {}", e),
                })?
                .to_str()
                .ok_or_else(|| HostApiError::AutoStartFailed {
                    reason: "无效的可执行文件路径".to_string(),
                })?
                .to_string();
            self.autostart_manager.enable(&task_name, &exe_path).await
        } else if self.autostart_manager.is_enabled(&task_name).await? {
            self.autostart_manager.disable(&task_name).await
        } else {
            Ok(())
        }
    }

    /// 检查自启动是否已启用
    ///
    /// 参数：无
    /// 返回：已启用返回 Ok(true)，否则返回 Ok(false)，失败返回 HostApiError
    pub async fn is_autostart_enabled(&self) -> Result<bool, HostApiError> {
        let task_name = self.autostart_manager.default_task_name();
        self.autostart_manager.is_enabled(&task_name).await
    }

    // ===== 按键监听服务 =====

    /// 应用按键配置。
    /// 注销所有现有快捷键，注册新快捷键，设置双击 Ctrl 状态。
    /// 参数：config - 按键配置。
    /// 返回：成功返回 Ok(())，失败返回 HostApiError。
    pub async fn apply_hotkey_config(&self, config: &HotkeyConfig) -> Result<(), HostApiError> {
        self.hotkey_manager.unregister_all().await?;
        for registration in &config.hotkeys {
            self.hotkey_manager
                .register_hotkey(&registration.hotkey)
                .await?;
        }
        self.hotkey_manager
            .set_double_ctrl_enabled(config.double_ctrl_enabled)
            .await?;
        Ok(())
    }

    /// 初始化按键监听。
    /// 将已注册的回调注入到 HotkeyManager，开始接收按键事件。
    /// 返回：成功返回 Ok(())，失败返回 HostApiError。
    pub async fn init_hotkey_listening(&self) -> Result<(), HostApiError> {
        self.hotkey_manager.start_listening().await
    }

    /// 检查快捷键是否正在监听。
    /// 参数：无。
    /// 返回：正在监听返回 true。
    pub fn is_hotkey_listening(&self) -> bool {
        self.hotkey_manager.is_listening()
    }

    // ===== 安装监控服务 =====

    /// 启动安装监控。
    /// 返回：成功返回 Ok(())，失败返回 HostApiError。
    pub async fn start_installation_monitor(&self) -> Result<(), HostApiError> {
        self.installation_monitor.start_watching().await
    }

    /// 停止安装监控。
    /// 返回：成功返回 Ok(())，失败返回 HostApiError。
    pub async fn stop_installation_monitor(&self) -> Result<(), HostApiError> {
        self.installation_monitor.stop_watching().await
    }

    /// 检查安装监控是否正在运行。
    pub fn is_installation_monitor_running(&self) -> bool {
        self.installation_monitor.is_watching()
    }

    /// 更新安装监控路径。
    /// 参数：paths - 要监控的目录路径列表。
    pub fn update_installation_monitor_paths(&self, paths: Vec<String>) {
        self.installation_monitor.update_watch_paths(paths);
    }

    // ===== 存储服务（宿主级） =====

    /// 获取当前存储服务的引用。
    /// 参数：无。
    /// 返回：当前存储服务的 Arc 引用。
    pub fn storage(&self) -> Arc<dyn StorageService> {
        let storage = self.storage.read().clone();
        storage.clone()
    }

    /// 重新配置存储服务（用户在设置中切换 Local/WebDAV 时调用）。
    /// 参数：new_service - 新的存储服务实例。
    /// 返回：无。
    /// 特性：立即生效，影响后续所有插件调用。
    pub fn reconfigure_storage(&self, new_service: Arc<dyn StorageService>) {
        *self.storage.write() = new_service;
    }
}

/// HostApi 构建器，用于链式配置平台组件并构建 HostApi 实例。
pub struct HostApiBuilder {
    icon_cache_dir: String,
    capabilities: Option<PlatformCapabilities>,
    icon_extractor: Option<Arc<dyn IconExtractor>>,
    shell_executor: Option<Arc<dyn ShellExecutor>>,
    window_manager: Option<Arc<dyn WindowManager>>,
    path_resolver: Option<Arc<dyn PathResolver>>,
    app_enumerator: Option<Arc<dyn AppEnumerator>>,
    app_launcher: Option<Arc<dyn AppLauncher>>,
    lnk_resolver: Option<Arc<dyn LnkResolver>>,
    resource_loader: Option<Arc<dyn ResourceLoader>>,
    parameter_resolver: Option<Arc<dyn ParameterResolver>>,
    clipboard_provider: Option<Arc<dyn SystemParameterProvider>>,
    window_handle_provider: Option<Arc<dyn SystemParameterProvider>>,
    selection_provider: Option<Arc<dyn SystemParameterProvider>>,
    autostart_manager: Option<Arc<dyn AutoStartManager>>,
    hotkey_manager: Option<Arc<dyn HotkeyManager>>,
    installation_monitor: Option<Arc<dyn InstallationMonitor>>,
    timer_manager: Option<Arc<dyn TimerManager>>,
    storage_service: Option<Arc<dyn StorageService>>,
    app_resource: Option<Arc<AppResourceService>>,
    focus_monitor: Option<Arc<dyn FocusMonitor>>,
    notify_callback: Option<Arc<dyn Fn(String, String) + Send + Sync + 'static>>,
    hide_window_callback: Option<Arc<dyn Fn() + Send + Sync + 'static>>,
    show_window_callback: Option<Arc<dyn Fn() + Send + Sync + 'static>>,
    is_window_visible_callback: Option<Arc<dyn Fn() -> bool + Send + Sync + 'static>>,
    window_positioner: Option<Arc<dyn WindowPositioner>>,
    set_window_position_callback: Option<Arc<dyn Fn(i32, i32) + Send + Sync + 'static>>,
}

impl HostApiBuilder {
    /// 创建 HostApiBuilder 实例。
    /// 参数：icon_cache_dir - 图标缓存目录。
    /// 返回：HostApiBuilder 实例。
    pub fn new(icon_cache_dir: String) -> Self {
        Self {
            icon_cache_dir,
            capabilities: None,
            icon_extractor: None,
            shell_executor: None,
            window_manager: None,
            path_resolver: None,
            app_enumerator: None,
            app_launcher: None,
            lnk_resolver: None,
            resource_loader: None,
            parameter_resolver: None,
            clipboard_provider: None,
            window_handle_provider: None,
            selection_provider: None,
            autostart_manager: None,
            hotkey_manager: None,
            installation_monitor: None,
            timer_manager: None,
            storage_service: None,
            app_resource: None,
            focus_monitor: None,
            notify_callback: None,
            hide_window_callback: None,
            show_window_callback: None,
            is_window_visible_callback: None,
            window_positioner: None,
            set_window_position_callback: None,
        }
    }

    /// 设置平台能力集。
    /// 参数：caps - 平台能力集合。
    /// 返回：Self（支持链式调用）。
    pub fn capabilities(mut self, caps: PlatformCapabilities) -> Self {
        self.capabilities = Some(caps);
        self
    }

    /// 设置图标提取器。
    /// 参数：icon_extractor - 图标提取器实例。
    /// 返回：Self（支持链式调用）。
    pub fn icon_extractor(mut self, icon_extractor: Arc<dyn IconExtractor>) -> Self {
        self.icon_extractor = Some(icon_extractor);
        self
    }

    /// 设置 Shell 执行器。
    /// 参数：shell_executor - Shell 执行器实例。
    /// 返回：Self（支持链式调用）。
    pub fn shell_executor(mut self, shell_executor: Arc<dyn ShellExecutor>) -> Self {
        self.shell_executor = Some(shell_executor);
        self
    }

    /// 设置窗口管理器。
    /// 参数：window_manager - 窗口管理器实例。
    /// 返回：Self（支持链式调用）。
    pub fn window_manager(mut self, window_manager: Arc<dyn WindowManager>) -> Self {
        self.window_manager = Some(window_manager);
        self
    }

    /// 设置路径解析器。
    /// 参数：path_resolver - 路径解析器实例。
    /// 返回：Self（支持链式调用）。
    pub fn path_resolver(mut self, path_resolver: Arc<dyn PathResolver>) -> Self {
        self.path_resolver = Some(path_resolver);
        self
    }

    /// 设置应用枚举器。
    /// 参数：app_enumerator - 应用枚举器实例。
    /// 返回：Self（支持链式调用）。
    pub fn app_enumerator(mut self, app_enumerator: Arc<dyn AppEnumerator>) -> Self {
        self.app_enumerator = Some(app_enumerator);
        self
    }

    /// 设置应用启动器。
    /// 参数：app_launcher - 应用启动器实例。
    /// 返回：Self（支持链式调用）。
    pub fn app_launcher(mut self, app_launcher: Arc<dyn AppLauncher>) -> Self {
        self.app_launcher = Some(app_launcher);
        self
    }

    /// 设置 Lnk 快捷方式解析器。
    /// 参数：lnk_resolver - Lnk 解析器实例。
    /// 返回：Self（支持链式调用）。
    pub fn lnk_resolver(mut self, lnk_resolver: Arc<dyn LnkResolver>) -> Self {
        self.lnk_resolver = Some(lnk_resolver);
        self
    }

    /// 设置资源加载器。
    /// 参数：resource_loader - 资源加载器实例。
    /// 返回：Self（支持链式调用）。
    pub fn resource_loader(mut self, resource_loader: Arc<dyn ResourceLoader>) -> Self {
        self.resource_loader = Some(resource_loader);
        self
    }

    /// 设置参数解析器。
    /// 参数：parameter_resolver - 参数解析器实例。
    /// 返回：Self（支持链式调用）。
    pub fn parameter_resolver(mut self, parameter_resolver: Arc<dyn ParameterResolver>) -> Self {
        self.parameter_resolver = Some(parameter_resolver);
        self
    }

    /// 设置参数提供者（剪贴板、窗口句柄、选中文本）。
    /// 参数：clipboard - 剪贴板提供者；window_handle - 窗口句柄提供者；selection - 选中文本提供者。
    /// 返回：Self（支持链式调用）。
    pub fn parameter_providers(
        mut self,
        clipboard: Arc<dyn SystemParameterProvider>,
        window_handle: Arc<dyn SystemParameterProvider>,
        selection: Arc<dyn SystemParameterProvider>,
    ) -> Self {
        self.clipboard_provider = Some(clipboard);
        self.window_handle_provider = Some(window_handle);
        self.selection_provider = Some(selection);
        self
    }

    /// 设置自启动管理器。
    /// 参数：autostart_manager - 自启动管理器实例。
    /// 返回：Self（支持链式调用）。
    pub fn autostart_manager(mut self, autostart_manager: Arc<dyn AutoStartManager>) -> Self {
        self.autostart_manager = Some(autostart_manager);
        self
    }

    /// 设置按键管理器。
    /// 参数：hotkey_manager - 按键管理器实例。
    /// 返回：Self（支持链式调用）。
    pub fn hotkey_manager(mut self, hotkey_manager: Arc<dyn HotkeyManager>) -> Self {
        self.hotkey_manager = Some(hotkey_manager);
        self
    }

    /// 设置安装监控器。
    /// 参数：installation_monitor - 安装监控器实例。
    /// 返回：Self（支持链式调用）。
    pub fn installation_monitor(
        mut self,
        installation_monitor: Arc<dyn InstallationMonitor>,
    ) -> Self {
        self.installation_monitor = Some(installation_monitor);
        self
    }

    /// 设置定时器管理器。
    /// 参数：timer_manager - 定时器管理器实例。
    /// 返回：Self（支持链式调用）。
    pub fn timer_manager(mut self, timer_manager: Arc<dyn TimerManager>) -> Self {
        self.timer_manager = Some(timer_manager);
        self
    }

    /// 设置存储服务。
    /// 参数：storage_service - 存储服务实例。
    /// 返回：Self（支持链式调用）。
    pub fn storage_service(mut self, storage_service: Arc<dyn StorageService>) -> Self {
        self.storage_service = Some(storage_service);
        self
    }

    /// 设置应用资源服务。
    /// 参数：app_resource - 应用资源服务实例。
    /// 返回：Self（支持链式调用）。
    pub fn app_resource(mut self, app_resource: Arc<AppResourceService>) -> Self {
        self.app_resource = Some(app_resource);
        self
    }

    /// 设置聚焦监控器
    /// 参数：focus_monitor - 聚焦监控器实例。
    /// 返回：Self（支持链式调用）。
    pub fn focus_monitor(mut self, focus_monitor: Arc<dyn FocusMonitor>) -> Self {
        self.focus_monitor = Some(focus_monitor);
        self
    }

    /// 设置通知回调，宿主层在初始化时注入平台通知实现。
    /// 参数：callback - 接收 (title, message) 的通知回调。
    /// 返回：Self（支持链式调用）。
    pub fn notify_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn(String, String) + Send + Sync + 'static,
    {
        self.notify_callback = Some(Arc::new(callback));
        self
    }

    /// 设置隐藏窗口回调，宿主层在初始化时注入 Tauri 窗口控制实现。
    /// 参数：callback - 隐藏搜索窗口的回调。
    /// 返回：Self（支持链式调用）。
    pub fn hide_window_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.hide_window_callback = Some(Arc::new(callback));
        self
    }

    /// 设置显示窗口回调，宿主层在初始化时注入 Tauri 窗口控制实现。
    /// 参数：callback - 显示搜索窗口的回调。
    /// 返回：Self（支持链式调用）。
    pub fn show_window_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.show_window_callback = Some(Arc::new(callback));
        self
    }

    /// 设置查询窗口可见性回调，宿主层在初始化时注入 Tauri 窗口查询实现。
    /// 参数：callback - 返回窗口是否可见的回调。
    /// 返回：Self（支持链式调用）。
    pub fn is_window_visible_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn() -> bool + Send + Sync + 'static,
    {
        self.is_window_visible_callback = Some(Arc::new(callback));
        self
    }

    /// 设置窗口位置计算器。
    /// 参数：positioner - 平台 WindowPositioner 实现。
    /// 返回：Self（支持链式调用）。
    pub fn window_positioner(mut self, positioner: Arc<dyn WindowPositioner>) -> Self {
        self.window_positioner = Some(positioner);
        self
    }

    /// 设置窗口位置回调，宿主层在初始化时注入 Tauri set_position 实现。
    /// 参数：callback - 接收 (x, y) 物理像素坐标的回调。
    /// 返回：Self（支持链式调用）。
    pub fn set_window_position_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn(i32, i32) + Send + Sync + 'static,
    {
        self.set_window_position_callback = Some(Arc::new(callback));
        self
    }

    /// 构建 HostApi 实例。
    /// 参数：无。
    /// 返回：构建完成的 HostApi 实例，如果缺少必需组件则返回 HostApiBuildError。
    pub fn build(self) -> Result<HostApi, HostApiBuildError> {
        let icon_cache = Arc::new(IconCacheService::new(self.icon_cache_dir));
        icon_cache.init();
        Ok(HostApi {
            handles: DashMap::new(),
            capabilities: self
                .capabilities
                .ok_or(HostApiBuildError::MissingComponent("capabilities"))?,
            icon_cache,
            icon_extractor: self
                .icon_extractor
                .ok_or(HostApiBuildError::MissingComponent("icon_extractor"))?,
            shell_executor: self
                .shell_executor
                .ok_or(HostApiBuildError::MissingComponent("shell_executor"))?,
            window_manager: self
                .window_manager
                .ok_or(HostApiBuildError::MissingComponent("window_manager"))?,
            path_resolver: self
                .path_resolver
                .ok_or(HostApiBuildError::MissingComponent("path_resolver"))?,
            app_enumerator: self
                .app_enumerator
                .ok_or(HostApiBuildError::MissingComponent("app_enumerator"))?,
            app_launcher: self
                .app_launcher
                .ok_or(HostApiBuildError::MissingComponent("app_launcher"))?,
            lnk_resolver: self
                .lnk_resolver
                .ok_or(HostApiBuildError::MissingComponent("lnk_resolver"))?,
            resource_loader: self
                .resource_loader
                .ok_or(HostApiBuildError::MissingComponent("resource_loader"))?,
            parameter_resolver: self
                .parameter_resolver
                .ok_or(HostApiBuildError::MissingComponent("parameter_resolver"))?,
            clipboard_provider: self
                .clipboard_provider
                .ok_or(HostApiBuildError::MissingComponent("clipboard_provider"))?,
            window_handle_provider: self.window_handle_provider.ok_or(
                HostApiBuildError::MissingComponent("window_handle_provider"),
            )?,
            selection_provider: self
                .selection_provider
                .ok_or(HostApiBuildError::MissingComponent("selection_provider"))?,
            autostart_manager: self
                .autostart_manager
                .ok_or(HostApiBuildError::MissingComponent("autostart_manager"))?,
            hotkey_manager: self
                .hotkey_manager
                .ok_or(HostApiBuildError::MissingComponent("hotkey_manager"))?,
            installation_monitor: self
                .installation_monitor
                .ok_or(HostApiBuildError::MissingComponent("installation_monitor"))?,
            timer_manager: self
                .timer_manager
                .ok_or(HostApiBuildError::MissingComponent("timer_manager"))?,
            storage: Arc::new(RwLock::new(
                self.storage_service
                    .ok_or(HostApiBuildError::MissingComponent("storage_service"))?,
            )),
            app_resource: self
                .app_resource
                .ok_or(HostApiBuildError::MissingComponent("app_resource"))?,
            focus_monitor: self
                .focus_monitor
                .ok_or(HostApiBuildError::MissingComponent("focus_monitor"))?,
            notify_callback: RwLock::new(
                self.notify_callback
                    .ok_or(HostApiBuildError::MissingComponent("notify_callback"))?,
            ),
            hide_window_callback: RwLock::new(
                self.hide_window_callback
                    .ok_or(HostApiBuildError::MissingComponent("hide_window_callback"))?,
            ),
            show_window_callback: RwLock::new(
                self.show_window_callback
                    .ok_or(HostApiBuildError::MissingComponent("show_window_callback"))?,
            ),
            is_window_visible_callback: RwLock::new(self.is_window_visible_callback.ok_or(
                HostApiBuildError::MissingComponent("is_window_visible_callback"),
            )?),
            window_positioner: self
                .window_positioner
                .ok_or(HostApiBuildError::MissingComponent("window_positioner"))?,
            set_window_position_callback: RwLock::new(self.set_window_position_callback.ok_or(
                HostApiBuildError::MissingComponent("set_window_position_callback"),
            )?),
        })
    }
}
