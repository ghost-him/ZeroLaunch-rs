use dashmap::DashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use zerolaunch_plugin_api::platform::{PlatformCapabilities, PlatformServices};
use zerolaunch_plugin_api::services::app::AppInfo;
use zerolaunch_plugin_api::services::focus_monitor::FocusCallback;
use zerolaunch_plugin_api::services::hotkey::types::{
    HotkeyCallback, HotkeyConfig, HotkeyEventFilter,
};
use zerolaunch_plugin_api::services::icon::icon_cache::IconCacheService;
use zerolaunch_plugin_api::services::installation_monitor::types::InstallationCallback;
use zerolaunch_plugin_api::services::model::{
    ModelChatRequest, ModelChatResponse, ModelEmbeddingRequest, ModelEmbeddingResponse, ModelError,
    ModelInfo, ModelService, ModelSimilarityRequest, ModelSimilarityResponse,
};
use zerolaunch_plugin_api::services::parameter::resolver::ParameterResolver;
use zerolaunch_plugin_api::services::parameter::types::ParameterSnapshot;
use zerolaunch_plugin_api::services::path::path_resolver::KnownPath;
use zerolaunch_plugin_api::services::resource::AppResourceService;
use zerolaunch_plugin_api::services::storage::storage_service::StorageService;
use zerolaunch_plugin_api::services::theme::Theme;
use zerolaunch_plugin_api::services::timer::{TimerCallback, TimerId, TimerManager, TimerMode};
use zerolaunch_plugin_api::services::window::{PositionRequest, WindowPosition};
use zerolaunch_plugin_api::services::IconRequest;

// Re-export from plugin-api
pub use zerolaunch_plugin_api::host::{
    build_cache_path, build_resource_path, CacheLevel, HostApiBuildError, HostApiError, OpenTarget,
    PluginHandle, PluginHost, PluginSdkConfig,
};

/// 宿主（进程内唯一执行节点）。
/// 持有平台端口（PlatformServices，由平台 crate 统一工厂注入）与宿主级服务
/// （storage/model/timer/parameter_resolver/app_resource/icon_cache/主题策略/窗口回调）。
/// 插件经 `register()` 获得只含身份/配置/能力集的 PluginHandle，句柄将全部操作
/// 委托回宿主（本类型实现的 PluginHost 契约）；宿主再决定直接处理或转发平台实现。
/// 全局管理操作（窗口控制、通知、热键配置、存储重配置等）保留在 HostApi 上。
pub struct HostApi {
    /// 已注册插件句柄表（键为 plugin_id）。
    ///
    /// 生命周期契约：句柄内部持有本宿主的强引用（Arc 环：宿主 → 句柄表 →
    /// 句柄 → 宿主）。注册后宿主不可被释放，`unregister`（配对调用，见
    /// register 文档）移除表项后环断开，宿主随最后外部引用释放。
    handles: DashMap<String, Arc<PluginHandle>>,
    /// 平台实现统一集合（平台端口），含能力集。
    platform: PlatformServices,
    /// 共享的图标缓存服务（宿主级，平台无关）
    icon_cache: Arc<IconCacheService>,
    /// 参数解析器（宿主级默认实现）
    parameter_resolver: Arc<dyn ParameterResolver>,
    /// 定时器管理器（宿主级 tokio 实现）
    timer_manager: Arc<dyn TimerManager>,
    /// 存储服务（宿主级，可运行时重配置：Local ↔ WebDAV）
    storage: Arc<RwLock<Arc<dyn StorageService>>>,
    /// 应用资源服务（宿主级）
    app_resource: Arc<AppResourceService>,
    /// 模型服务（聚合所有模型提供方并按 model_id 路由）
    model_service: Arc<dyn ModelService>,
    /// 宿主当前主题配置模式（system/light/dark）
    theme_mode: Arc<RwLock<String>>,
    /// 通知回调（宿主级）
    notify_callback: RwLock<Arc<dyn Fn(String, String) + Send + Sync + 'static>>,
    /// 隐藏窗口回调（宿主级）
    hide_window_callback: RwLock<Arc<dyn Fn() + Send + Sync + 'static>>,
    /// 显示窗口回调（宿主级）
    show_window_callback: RwLock<Arc<dyn Fn() + Send + Sync + 'static>>,
    /// 查询窗口可见性回调（宿主级）
    is_window_visible_callback: RwLock<Arc<dyn Fn() -> bool + Send + Sync + 'static>>,
    /// 设置窗口位置回调（宿主级）
    set_window_position_callback: RwLock<Arc<dyn Fn(i32, i32) + Send + Sync + 'static>>,
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
    /// 返回：绑定该插件身份的接口层句柄。
    /// 特性：同一 plugin_id 重复注册将覆盖原有句柄；句柄不持有任何服务实现，
    /// 全部操作经 PluginHost 契约委托回宿主执行。
    /// 生命周期：句柄与本宿主经 Arc 互持（引用环）。释放宿主前必须对每个已
    /// 注册 plugin_id 调用 `unregister` 断开环，否则宿主与句柄均不释放。
    pub fn register(
        self: &Arc<Self>,
        plugin_id: &str,
        config: PluginSdkConfig,
    ) -> Arc<PluginHandle> {
        let handle = Arc::new(PluginHandle::new(
            plugin_id.to_string(),
            config,
            self.platform.capabilities.clone(),
            self.clone() as Arc<dyn PluginHost>,
        ));
        self.handles.insert(plugin_id.to_string(), handle.clone());
        handle
    }

    /// 获取已注册插件的句柄。
    pub fn get_plugin_handle(&self, plugin_id: &str) -> Option<Arc<PluginHandle>> {
        self.handles
            .get(plugin_id)
            .map(|entry| entry.value().clone())
    }

    /// 注销插件句柄（断开 register 建立的宿主↔句柄引用环）。
    pub fn unregister(&self, plugin_id: &str) {
        self.handles.remove(plugin_id);
    }

    /// 更新图标缓存目录路径（宿主级操作，不暴露给插件）。
    /// 参数：new_icon_cache_dir - 新的图标文件缓存目录路径。
    /// 返回：成功返回 Ok(())，失败返回 HostApiError。
    /// 特性：切换文件缓存的存储位置，同时清空内存缓存以保持一致性。
    pub fn update_icon_cache_dir(&self, new_icon_cache_dir: &str) -> Result<(), HostApiError> {
        self.icon_cache.update_cache_dir(new_icon_cache_dir);
        Ok(())
    }

    /// 预载图标文件缓存到内存（L1），消除冷启动后首次查询的磁盘读。
    /// 参数：无。
    /// 返回：无。
    /// 特性：启动阶段调用一次；并行读取当前格式条目，失败条目静默跳过。
    pub async fn preload_icon_cache(&self) {
        self.icon_cache.preload_l1().await;
    }

    /// 查询当前平台支持的能力集合。
    /// 参数：无。
    /// 返回：平台能力的不可变引用。
    pub fn capabilities(&self) -> &PlatformCapabilities {
        &self.platform.capabilities
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

    /// 更新宿主主题配置模式。
    /// 参数：mode - `system`、`light` 或 `dark`；非法值回退为 `system`。
    pub fn set_theme_mode(&self, mode: &str) {
        let normalized = match mode {
            "light" | "dark" | "system" => mode,
            _ => "system",
        };
        *self.theme_mode.write() = normalized.to_string();
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
        request: PositionRequest,
    ) -> Result<WindowPosition, HostApiError> {
        self.platform
            .window_positioner
            .compute_position(request)
            .await
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

        if let Ok(value) = self.platform.clipboard_provider.get_value().await {
            snapshot.insert("clipboard".to_string(), value);
        }

        if let Ok(value) = self.platform.window_handle_provider.get_value().await {
            snapshot.insert("hwnd".to_string(), value);
        }

        if let Ok(value) = self.platform.selection_provider.get_value().await {
            snapshot.insert("selection".to_string(), value);
        }

        snapshot
    }

    // ===== 自启动服务 =====

    /// 应用自启动设置。根据 enabled 启用或禁用自启动。
    ///
    /// 参数：enabled - 是否启用自启动
    /// 返回：成功返回 Ok(())，失败返回 HostApiError
    pub async fn apply_autostart_setting(&self, enabled: bool) -> Result<(), HostApiError> {
        let task_name = self.platform.autostart_manager.default_task_name();
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
            self.platform
                .autostart_manager
                .enable(&task_name, &exe_path)
                .await
        } else if self
            .platform
            .autostart_manager
            .is_enabled(&task_name)
            .await?
        {
            self.platform.autostart_manager.disable(&task_name).await
        } else {
            Ok(())
        }
    }

    /// 检查自启动是否已启用
    ///
    /// 参数：无
    /// 返回：已启用返回 Ok(true)，否则返回 Ok(false)，失败返回 HostApiError
    pub async fn is_autostart_enabled(&self) -> Result<bool, HostApiError> {
        let task_name = self.platform.autostart_manager.default_task_name();
        self.platform.autostart_manager.is_enabled(&task_name).await
    }

    // ===== 按键监听服务 =====

    /// 应用按键配置。
    /// 注销所有现有快捷键，注册新快捷键，设置双击 Ctrl 状态。
    /// 参数：config - 按键配置。
    /// 返回：成功返回 Ok(())，失败返回 HostApiError。
    pub async fn apply_hotkey_config(&self, config: &HotkeyConfig) -> Result<(), HostApiError> {
        self.platform.hotkey_manager.unregister_all().await?;
        for registration in &config.hotkeys {
            self.platform
                .hotkey_manager
                .register_hotkey(&registration.hotkey)
                .await?;
        }
        self.platform
            .hotkey_manager
            .set_double_ctrl_enabled(config.double_ctrl_enabled)
            .await?;
        Ok(())
    }

    /// 注销所有已注册的快捷键并禁用双击 Ctrl。
    /// 用于游戏模式等场景，临时禁用所有全局快捷键。
    /// 返回：成功返回 Ok(())，失败返回 HostApiError。
    pub async fn unregister_all_hotkeys(&self) -> Result<(), HostApiError> {
        self.platform.hotkey_manager.unregister_all().await?;
        self.platform
            .hotkey_manager
            .set_double_ctrl_enabled(false)
            .await?;
        Ok(())
    }

    /// 初始化按键监听。
    /// 将已注册的回调注入到 HotkeyManager，开始接收按键事件。
    /// 返回：成功返回 Ok(())，失败返回 HostApiError。
    pub async fn init_hotkey_listening(&self) -> Result<(), HostApiError> {
        self.platform.hotkey_manager.start_listening().await
    }

    /// 检查快捷键是否正在监听。
    /// 参数：无。
    /// 返回：正在监听返回 true。
    pub fn is_hotkey_listening(&self) -> bool {
        self.platform.hotkey_manager.is_listening()
    }

    // ===== 安装监控服务 =====

    /// 启动安装监控。
    /// 返回：成功返回 Ok(())，失败返回 HostApiError。
    pub async fn start_installation_monitor(&self) -> Result<(), HostApiError> {
        self.platform.installation_monitor.start_watching().await
    }

    /// 停止安装监控。
    /// 返回：成功返回 Ok(())，失败返回 HostApiError。
    pub async fn stop_installation_monitor(&self) -> Result<(), HostApiError> {
        self.platform.installation_monitor.stop_watching().await
    }

    /// 检查安装监控是否正在运行。
    pub fn is_installation_monitor_running(&self) -> bool {
        self.platform.installation_monitor.is_watching()
    }

    /// 更新安装监控路径。
    /// 参数：paths - 要监控的目录路径列表（为空时平台层回退默认开始菜单路径）。
    pub fn update_installation_monitor_paths(&self, paths: Vec<String>) {
        self.platform.installation_monitor.update_watch_paths(paths);
    }

    /// 更新安装监控去抖时间。
    /// 参数：secs - 事件静默满该秒数后才触发回调（配置组件 monitor_debounce_secs，范围 1-60）。
    pub fn update_installation_monitor_debounce(&self, secs: f64) {
        self.platform
            .installation_monitor
            .update_debounce_secs(secs);
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

/// 宿主实现 PluginHost 契约：插件全部可调用操作在此处理。
/// 平台面操作（shell/窗口/图标/路径/剪贴板/应用/lnk/主题查询）转发到平台端口；
/// 宿主面操作（模型/存储/定时器/参数/资源与缓存规约/回调注册）直接处理。
#[async_trait::async_trait]
impl PluginHost for HostApi {
    // ===== 图标服务 =====

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
                tracing::warn!("图标提取失败，使用默认图标: {:?}", request);
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
        original_request: &IconRequest,
        custom_icon_path: &str,
    ) -> Result<(), HostApiError> {
        // 缓存键后缀与提取/查询链路一致（.webp）
        let hash_key = original_request.get_hash_string() + ".webp";

        // 从自定义文件提取并处理图标
        let custom_request = IconRequest::Path(custom_icon_path.to_string());
        let data = self
            .platform
            .icon_extractor
            .extract_and_process(&custom_request)
            .await?;

        // 覆盖写入 L1 + L2 缓存
        self.icon_cache.set_l1(&hash_key, data.clone());
        self.icon_cache.set_l2(&hash_key, data).await;

        Ok(())
    }

    // ===== Shell 服务 =====

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

    // ===== 窗口服务 =====

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

    // ===== 路径服务 =====

    fn resolve_path(&self, path: KnownPath) -> Result<String, HostApiError> {
        self.platform.path_resolver.resolve_path(path)
    }

    // ===== 剪贴板服务 =====

    fn set_clipboard_text(&self, text: &str) -> Result<(), HostApiError> {
        self.platform.clipboard_manager.set_text(text)
    }

    // ===== 应用服务 =====

    async fn enumerate_apps(&self) -> Vec<AppInfo> {
        self.platform.app_enumerator.enumerate_apps().await
    }

    async fn launch_app(&self, app_id: &str, args: Option<&[String]>) -> Result<u32, HostApiError> {
        self.platform.app_launcher.launch_app(app_id, args).await
    }

    // ===== 应用资源服务 =====

    fn get_app_icon_path(&self, name: &str) -> Option<String> {
        self.app_resource.get_icon_path(name)
    }

    // ===== 快捷方式解析 =====

    fn resolve_lnk_target(&self, lnk_path: &str) -> Option<String> {
        self.platform.lnk_resolver.resolve_lnk_target(lnk_path)
    }

    fn parse_localized_names_from_dir(
        &self,
        dir_path: &std::path::Path,
    ) -> std::collections::HashMap<String, String> {
        self.platform
            .resource_loader
            .parse_localized_names_from_dir(dir_path)
    }

    // ===== 主题服务 =====

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

    // ===== 模型服务 =====

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

    // ===== 参数解析服务 =====

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

    // ===== 定时器服务 =====

    async fn set_timeout(
        &self,
        delay: std::time::Duration,
        callback: TimerCallback,
    ) -> Result<TimerId, HostApiError> {
        self.timer_manager
            .set_timer(delay, TimerMode::OneShot, callback)
            .await
    }

    async fn set_interval(
        &self,
        interval: std::time::Duration,
        callback: TimerCallback,
    ) -> Result<TimerId, HostApiError> {
        self.timer_manager
            .set_timer(interval, TimerMode::Interval, callback)
            .await
    }

    async fn cancel_timer(&self, id: TimerId) -> Result<(), HostApiError> {
        self.timer_manager.cancel_timer(id).await
    }

    async fn cancel_all_timers(&self) -> Result<(), HostApiError> {
        self.timer_manager.cancel_all().await
    }

    // ===== 资源管理（插件作用域） =====

    async fn resource_upload(
        &self,
        plugin_id: &str,
        resource_id: &str,
        file_path: &str,
        max_size: Option<u64>,
    ) -> Result<String, HostApiError> {
        let path = std::path::Path::new(file_path);

        if let Some(limit) = max_size {
            let metadata = tokio::fs::metadata(path).await.map_err(|e| {
                HostApiError::StorageOperationFailed {
                    file: file_path.to_string(),
                    reason: format!("读取文件元数据失败: {}", e),
                }
            })?;
            if metadata.len() > limit {
                return Err(HostApiError::StorageOperationFailed {
                    file: file_path.to_string(),
                    reason: format!("文件大小 {} 超过限制 {} 字节", metadata.len(), limit),
                });
            }
        }

        let data =
            tokio::fs::read(path)
                .await
                .map_err(|e| HostApiError::StorageOperationFailed {
                    file: file_path.to_string(),
                    reason: format!("读取文件失败: {}", e),
                })?;

        // 直接使用 resource_id 作为存储标识符，避免对用户指定的标识符做额外变换。
        let storage_path = build_resource_path(plugin_id, Some(resource_id))?;
        let storage = self.storage.read().clone();
        storage.upload(&storage_path, &data).await.map_err(|e| {
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
        let storage: Arc<dyn StorageService> = self.storage.read().clone();
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
                file: path,
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

    // ===== 本地缓存（插件作用域） =====

    async fn cache_put(
        &self,
        plugin_id: &str,
        domain: &str,
        key: &str,
        data: &[u8],
    ) -> Result<(), HostApiError> {
        let cache_root = self
            .platform
            .path_resolver
            .resolve_path(KnownPath::AppCacheDir)?;
        let path = build_cache_path(&cache_root, plugin_id, domain, key)?;
        if let Some(parent) = std::path::Path::new(&path).parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                HostApiError::StorageOperationFailed {
                    file: path.clone(),
                    reason: format!("创建缓存目录失败: {}", e),
                }
            })?;
        }
        tokio::fs::write(&path, data)
            .await
            .map_err(|e| HostApiError::StorageOperationFailed {
                file: path,
                reason: format!("写入缓存文件失败: {}", e),
            })?;
        Ok(())
    }

    async fn cache_get(
        &self,
        plugin_id: &str,
        domain: &str,
        key: &str,
    ) -> Result<Option<Vec<u8>>, HostApiError> {
        let cache_root = self
            .platform
            .path_resolver
            .resolve_path(KnownPath::AppCacheDir)?;
        let path = build_cache_path(&cache_root, plugin_id, domain, key)?;
        match tokio::fs::read(&path).await {
            Ok(data) => Ok(Some(data)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(HostApiError::StorageOperationFailed {
                file: path,
                reason: format!("读取缓存文件失败: {}", e),
            }),
        }
    }

    async fn cache_delete(
        &self,
        plugin_id: &str,
        domain: &str,
        key: &str,
    ) -> Result<(), HostApiError> {
        let cache_root = self
            .platform
            .path_resolver
            .resolve_path(KnownPath::AppCacheDir)?;
        let path = build_cache_path(&cache_root, plugin_id, domain, key)?;
        match tokio::fs::remove_file(&path).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(HostApiError::StorageOperationFailed {
                file: path,
                reason: format!("删除缓存文件失败: {}", e),
            }),
        }
    }

    async fn cache_cleanup(
        &self,
        plugin_id: &str,
        domain: &str,
        max_entries: usize,
    ) -> Result<(), HostApiError> {
        let cache_root = self
            .platform
            .path_resolver
            .resolve_path(KnownPath::AppCacheDir)?;
        let domain_dir = std::path::Path::new(&cache_root)
            .join(plugin_id)
            .join(domain);
        if !domain_dir.exists() {
            return Ok(());
        }

        // 收集全部条目（含两级分片目录）及其修改时间
        let mut entries: Vec<(std::time::SystemTime, std::path::PathBuf)> = Vec::new();
        for sub in
            std::fs::read_dir(&domain_dir).map_err(|e| HostApiError::StorageOperationFailed {
                file: domain_dir.to_string_lossy().to_string(),
                reason: format!("读取缓存目录失败: {}", e),
            })?
        {
            let sub = sub.map_err(|e| HostApiError::StorageOperationFailed {
                file: domain_dir.to_string_lossy().to_string(),
                reason: format!("读取缓存子目录失败: {}", e),
            })?;
            if !sub.path().is_dir() {
                continue;
            }
            for file in
                std::fs::read_dir(sub.path()).map_err(|e| HostApiError::StorageOperationFailed {
                    file: sub.path().to_string_lossy().to_string(),
                    reason: format!("读取缓存分片目录失败: {}", e),
                })?
            {
                let file = file.map_err(|e| HostApiError::StorageOperationFailed {
                    file: sub.path().to_string_lossy().to_string(),
                    reason: format!("读取缓存条目失败: {}", e),
                })?;
                let meta = file
                    .metadata()
                    .map_err(|e| HostApiError::StorageOperationFailed {
                        file: file.path().to_string_lossy().to_string(),
                        reason: format!("读取缓存条目元数据失败: {}", e),
                    })?;
                if meta.is_file() {
                    entries.push((
                        meta.modified().unwrap_or(std::time::UNIX_EPOCH),
                        file.path(),
                    ));
                }
            }
        }

        if entries.len() <= max_entries {
            return Ok(());
        }
        entries.sort_by_key(|(mtime, _)| *mtime);
        let excess = entries.len() - max_entries;
        for (_, path) in entries.into_iter().take(excess) {
            let _ = std::fs::remove_file(&path);
        }
        Ok(())
    }

    // ===== 推送式回调注册（插件作用域） =====

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

/// HostApi 构建器，用于链式配置平台端口与宿主级服务并构建 HostApi 实例。
pub struct HostApiBuilder {
    icon_cache_dir: String,
    platform: Option<PlatformServices>,
    parameter_resolver: Option<Arc<dyn ParameterResolver>>,
    timer_manager: Option<Arc<dyn TimerManager>>,
    storage_service: Option<Arc<dyn StorageService>>,
    app_resource: Option<Arc<AppResourceService>>,
    model_service: Option<Arc<dyn ModelService>>,
    notify_callback: Option<Arc<dyn Fn(String, String) + Send + Sync + 'static>>,
    hide_window_callback: Option<Arc<dyn Fn() + Send + Sync + 'static>>,
    show_window_callback: Option<Arc<dyn Fn() + Send + Sync + 'static>>,
    is_window_visible_callback: Option<Arc<dyn Fn() -> bool + Send + Sync + 'static>>,
    set_window_position_callback: Option<Arc<dyn Fn(i32, i32) + Send + Sync + 'static>>,
}

impl HostApiBuilder {
    /// 创建 HostApiBuilder 实例。
    /// 参数：icon_cache_dir - 图标缓存目录。
    /// 返回：HostApiBuilder 实例。
    pub fn new(icon_cache_dir: String) -> Self {
        Self {
            icon_cache_dir,
            platform: None,
            parameter_resolver: None,
            timer_manager: None,
            storage_service: None,
            app_resource: None,
            model_service: None,
            notify_callback: None,
            hide_window_callback: None,
            show_window_callback: None,
            is_window_visible_callback: None,
            set_window_position_callback: None,
        }
    }

    /// 注入平台端口（PlatformServices）。
    /// 由平台 crate 的统一工厂（如 windows_platform_services）或 mock 桩构造。
    /// 参数：services - 平台实现统一集合。
    /// 返回：Self（支持链式调用）。
    pub fn platform(mut self, services: PlatformServices) -> Self {
        self.platform = Some(services);
        self
    }

    /// 设置参数解析器（宿主级默认实现，非平台面）。
    /// 参数：parameter_resolver - 参数解析器实例。
    /// 返回：Self（支持链式调用）。
    pub fn parameter_resolver(mut self, parameter_resolver: Arc<dyn ParameterResolver>) -> Self {
        self.parameter_resolver = Some(parameter_resolver);
        self
    }

    /// 设置定时器管理器（宿主级 tokio 实现，非平台面）。
    /// 参数：timer_manager - 定时器管理器实例。
    /// 返回：Self（支持链式调用）。
    pub fn timer_manager(mut self, timer_manager: Arc<dyn TimerManager>) -> Self {
        self.timer_manager = Some(timer_manager);
        self
    }

    /// 设置存储服务（宿主级，Local/WebDAV 由宿主装配）。
    /// 参数：storage_service - 存储服务实例。
    /// 返回：Self（支持链式调用）。
    pub fn storage_service(mut self, storage_service: Arc<dyn StorageService>) -> Self {
        self.storage_service = Some(storage_service);
        self
    }

    /// 设置应用资源服务（宿主级）。
    /// 参数：app_resource - 应用资源服务实例。
    /// 返回：Self（支持链式调用）。
    pub fn app_resource(mut self, app_resource: Arc<AppResourceService>) -> Self {
        self.app_resource = Some(app_resource);
        self
    }

    /// 设置模型服务（宿主级）。
    /// 参数：model_service - 模型服务实例。
    /// 返回：Self（支持链式调用）。
    pub fn model_service(mut self, model_service: Arc<dyn ModelService>) -> Self {
        self.model_service = Some(model_service);
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
            platform: self
                .platform
                .ok_or(HostApiBuildError::MissingComponent("platform"))?,
            icon_cache,
            parameter_resolver: self
                .parameter_resolver
                .ok_or(HostApiBuildError::MissingComponent("parameter_resolver"))?,
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
            model_service: self
                .model_service
                .ok_or(HostApiBuildError::MissingComponent("model_service"))?,
            theme_mode: Arc::new(RwLock::new("system".to_string())),
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
            set_window_position_callback: RwLock::new(self.set_window_position_callback.ok_or(
                HostApiBuildError::MissingComponent("set_window_position_callback"),
            )?),
        })
    }
}
