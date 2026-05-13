use crate::sdk::app::app_enumerator::AppEnumerator;
use crate::sdk::app::app_launcher::AppLauncher;
use crate::sdk::app::AppInfo;
use crate::sdk::autostart::AutoStartManager;
use crate::sdk::focus_monitor::types::FocusCallback;
use crate::sdk::focus_monitor::FocusMonitor;
use crate::sdk::hotkey::types::{HotkeyCallback, HotkeyConfig, HotkeyEventFilter};
use crate::sdk::hotkey::HotkeyManager;
use crate::sdk::icon::icon_cache::IconCacheService;
use crate::sdk::icon::icon_extractor::IconExtractor;
use crate::sdk::installation_monitor::types::InstallationCallback;
use crate::sdk::installation_monitor::InstallationMonitor;
use crate::sdk::parameter::provider::SystemParameterProvider;
use crate::sdk::parameter::resolver::ParameterResolver;
use crate::sdk::parameter::types::ParameterSnapshot;
use crate::sdk::path::path_resolver::{KnownPath, PathResolver};
use crate::sdk::platform::capabilities::PlatformCapabilities;
use crate::sdk::resource::AppResourceService;
use crate::sdk::shell::lnk_resolver::LnkResolver;
use crate::sdk::shell::resource_loader::ResourceLoader;
use crate::sdk::shell::ShellExecutor;
use crate::sdk::storage::storage_service::StorageService;
use crate::sdk::timer::types::{TimerCallback, TimerId, TimerMode};
use crate::sdk::timer::TimerManager;
use crate::sdk::window::WindowManager;
use dashmap::DashMap;
use parking_lot::RwLock;

use std::sync::Arc;

use crate::sdk::icon::IconRequest;

/// Shell 打开目标类型。
/// 将不同打开语义统一为枚举，平台层根据类型选择不同的系统调用。
#[derive(Debug, Clone)]
pub enum OpenTarget {
    /// 使用系统默认程序打开文件
    File(String),
    /// 使用默认浏览器打开网址
    Url(String),
    /// 使用文件资源管理器打开文件夹
    Folder(String),
}

/// 缓存等级枚举，控制图标服务的缓存策略。
/// 插件通过 PluginSdkConfig 注册时指定，HostApi 根据等级决定缓存行为。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CacheLevel {
    /// 双层缓存（L1 → L2 → 提取 → 更新 L1+L2）。
    /// 适用于图标被频繁提取的场景。
    #[default]
    Full,
    /// 跳过内存缓存（L2 → 提取 → 更新 L2）。
    /// 适用于图标只在每次启动时提取的场景。
    SkipMemory,
    /// 跳过所有缓存（直接提取）。
    /// 适用于图标在几天的时间内可能只被提取一次的场景。
    SkipAll,
}

/// 插件 SDK 配置。
/// 各字段可选，不需要配置的服务无需设置，使用默认值。
#[derive(Debug, Clone, Default)]
pub struct PluginSdkConfig {
    /// 图标缓存等级。None 时使用默认值 CacheLevel::Full。
    pub icon_cache_level: Option<CacheLevel>,
}

/// HostApi 统一错误类型。
/// 涵盖能力查询失败、注册失败、各服务操作失败等场景。
#[derive(Debug, thiserror::Error)]
pub enum HostApiError {
    /// 当前平台不支持请求的能力
    #[error("平台不支持该能力: {0:?}")]
    UnsupportedCapability(crate::sdk::platform::capabilities::PlatformCapability),

    /// 插件未注册或 plugin_id 不存在
    #[error("插件未注册: {0}")]
    PluginNotRegistered(String),

    /// 图标提取失败
    #[error("图标提取失败 (请求: {request}): {reason}")]
    IconExtractionFailed { request: String, reason: String },

    /// Shell 操作失败
    #[error("Shell 操作失败 (目标: {target}): {reason}")]
    ShellOperationFailed { target: String, reason: String },

    /// 窗口操作失败
    #[error("窗口操作失败: {detail}")]
    WindowOperationFailed { detail: String },

    /// 通用执行失败
    #[error("{service} 执行失败: {reason}")]
    ExecutionFailed { service: String, reason: String },

    /// 路径解析失败
    #[error("路径解析失败 ({path}): {reason}")]
    PathResolutionFailed { path: String, reason: String },

    /// 应用枚举失败
    #[error("应用枚举失败: {reason}")]
    AppEnumerationFailed { reason: String },

    /// 应用启动失败
    #[error("应用启动失败 (app_id: {app_id}): {reason}")]
    AppLaunchFailed { app_id: String, reason: String },

    /// Lnk 快捷方式解析失败
    #[error("Lnk 解析失败 ({path}): {reason}")]
    LnkResolutionFailed { path: String, reason: String },

    /// 参数解析失败
    #[error("参数解析失败: {reason}")]
    ParameterResolutionFailed { reason: String },

    /// 自启动操作失败
    #[error("自启动操作失败: {reason}")]
    AutoStartFailed { reason: String },

    /// 存储操作失败
    #[error("存储操作失败 ({file}): {reason}")]
    StorageOperationFailed { file: String, reason: String },

    /// 资源未找到
    #[error("资源未找到: {id}")]
    ResourceNotFound { id: String },
}

/// 插件服务句柄，绑定插件身份与配置。
/// 跨平台 struct，通过 Arc<dyn IconExtractor> 等平台 trait 注入平台代码。
/// 插件通过 HostApi::register() 获取此句柄，后续所有服务调用通过句柄完成。
/// 句柄自动应用注册时的插件配置（如缓存等级），插件无需在每次调用时传递配置。
pub struct PluginHandle {
    #[allow(dead_code)]
    plugin_id: String,
    config: RwLock<PluginSdkConfig>,
    capabilities: PlatformCapabilities,
    /// 图标提取器，由 HostApi 注入的平台实现
    icon_extractor: Arc<dyn IconExtractor>,
    /// 图标缓存服务，由 HostApi 共享
    icon_cache: Arc<IconCacheService>,
    /// Shell 执行器，由 HostApi 注入的平台实现
    shell_executor: Arc<dyn ShellExecutor>,
    /// 窗口管理器，由 HostApi 注入的平台实现
    window_manager: Arc<dyn WindowManager>,
    /// 路径解析器，由 HostApi 注入的平台实现
    path_resolver: Arc<dyn PathResolver>,
    /// 应用枚举器，由 HostApi 注入的平台实现
    app_enumerator: Arc<dyn AppEnumerator>,
    /// 应用启动器，由 HostApi 注入的平台实现
    app_launcher: Arc<dyn AppLauncher>,
    /// Lnk 快捷方式解析器，由 HostApi 注入的平台实现
    lnk_resolver: Arc<dyn LnkResolver>,
    /// 资源加载器，由 HostApi 注入的平台实现
    resource_loader: Arc<dyn ResourceLoader>,
    /// 参数解析器，由 HostApi 注入
    parameter_resolver: Arc<dyn ParameterResolver>,
    /// 定时器管理器，由 HostApi 注入
    timer_manager: Arc<dyn TimerManager>,
    /// 应用资源服务，由 HostApi 注入
    app_resource: Arc<AppResourceService>,
    /// 存储服务，由 HostApi 注入
    storage: Arc<dyn StorageService>,
    /// 按键管理器，由 HostApi 注入
    hotkey_manager: Arc<dyn HotkeyManager>,
    /// 安装监控器，由 HostApi 注入
    installation_monitor: Arc<dyn InstallationMonitor>,
    /// 聚焦监控器，由 HostApi 注入
    focus_monitor: Arc<dyn FocusMonitor>,
}

impl PluginHandle {
    /// 获取当前图标缓存等级，None 时返回默认值 Full。
    fn icon_cache_level(&self) -> CacheLevel {
        self.config.read().icon_cache_level.unwrap_or_default()
    }

    // ===== 图标服务 =====

    /// 根据图标请求提取图标数据，行为由注册时的缓存等级决定。
    /// 参数：request - 图标请求（路径/网址/扩展名）。
    /// 返回：PNG 格式的图标字节数据，失败返回 HostApiError。
    pub async fn get_icon(&self, request: IconRequest) -> Result<Vec<u8>, HostApiError> {
        let level = self.icon_cache_level();
        self.icon_extractor
            .get_icon(&self.icon_cache, &request, level)
            .await
    }

    /// 提取图标数据，失败时回退到默认图标。
    /// 与 get_icon 不同，此方法永不返回错误，提取失败时返回默认图标数据。
    pub async fn get_icon_or_default(&self, request: IconRequest) -> Vec<u8> {
        let level = self.icon_cache_level();
        match self
            .icon_extractor
            .get_icon(&self.icon_cache, &request, level)
            .await
        {
            Ok(data) if !data.is_empty() => data,
            _ => {
                tracing::warn!("图标提取失败，使用默认图标: {:?}", request);
                self.icon_extractor.load_default_icon(&request).await
            }
        }
    }

    /// 强制从磁盘提取图标数据并根据缓存等级更新缓存。
    /// 与 get_icon 不同，此方法跳过缓存读取，直接提取并更新缓存。
    /// 参数：request - 图标请求（路径/网址/扩展名）。
    /// 返回：PNG 格式的图标字节数据，失败返回 HostApiError。
    pub async fn get_icon_and_update_cache(
        &self,
        request: IconRequest,
    ) -> Result<Vec<u8>, HostApiError> {
        let level = self.icon_cache_level();
        self.icon_extractor
            .get_icon_and_update_cache(&self.icon_cache, &request, level)
            .await
    }

    // ===== Shell 服务 =====

    /// 使用系统默认方式打开目标（文件/网址/文件夹）。
    /// 参数：target - 打开目标。
    /// 返回：成功返回 Ok(())，失败返回 HostApiError。
    pub async fn shell_open(&self, target: OpenTarget) -> Result<(), HostApiError> {
        self.shell_executor.shell_open(&target).await
    }

    /// 在文件资源管理器中打开指定路径的父目录并选中该文件。
    /// 参数：path - 要打开所在位置的文件路径。
    /// 返回：成功返回 Ok(())，失败返回 HostApiError。
    pub async fn shell_open_folder(&self, path: &str) -> Result<(), HostApiError> {
        self.shell_executor.shell_open_folder(path).await
    }

    /// 以管理员权限启动程序。
    /// 参数：path - 要以管理员身份运行的程序路径。
    /// 返回：成功返回 Ok(())，失败返回 HostApiError。
    pub async fn shell_execute_elevation(&self, path: &str) -> Result<(), HostApiError> {
        self.shell_executor.shell_execute_elevation(path).await
    }

    /// 执行命令字符串（后台运行，无窗口）。
    /// 参数：command - 要执行的命令字符串。
    /// 返回：成功返回 Ok(())，失败返回 HostApiError。
    pub async fn shell_execute_command(&self, command: &str) -> Result<(), HostApiError> {
        self.shell_executor.shell_execute_command(command).await
    }

    // ===== 窗口服务 =====

    /// 根据进程名（如 "chrome.exe"）激活已存在的窗口。
    /// 参数：process_name - 进程名（含扩展名）。
    /// 返回：成功激活返回 Ok(true)，未找到窗口返回 Ok(false)，失败返回 HostApiError。
    pub async fn activate_window_by_process(
        &self,
        process_name: &str,
    ) -> Result<bool, HostApiError> {
        self.window_manager
            .activate_window_by_process(process_name)
            .await
    }

    /// 根据窗口标题的部分内容激活已存在的窗口。
    /// 参数：title - 窗口标题的部分匹配文本。
    /// 返回：成功激活返回 Ok(true)，未找到窗口返回 Ok(false)，失败返回 HostApiError。
    pub async fn activate_window_by_title(&self, title: &str) -> Result<bool, HostApiError> {
        self.window_manager.activate_window_by_title(title).await
    }

    // ===== 路径服务 =====

    /// 根据已知路径类型解析实际文件系统路径。
    /// 参数：path - 已知路径类型枚举。
    /// 返回：解析后的路径字符串，失败返回 HostApiError。
    pub fn resolve_path(&self, path: KnownPath) -> Result<String, HostApiError> {
        self.path_resolver.resolve_path(path)
    }

    // ===== 应用服务 =====

    /// 枚举当前平台已安装的应用。
    /// 参数：无。
    /// 返回：应用信息列表。
    pub async fn enumerate_apps(&self) -> Vec<AppInfo> {
        self.app_enumerator.enumerate_apps().await
    }

    /// 启动指定应用。
    /// 参数：app_id - 应用唯一标识；args - 启动参数（可选）。
    /// 返回：成功返回 Ok(pid)，失败返回 HostApiError。
    pub async fn launch_app(
        &self,
        app_id: &str,
        args: Option<&[String]>,
    ) -> Result<u32, HostApiError> {
        self.app_launcher.launch_app(app_id, args).await
    }

    // ===== 应用资源服务 =====

    /// 根据名称获取内置图标资源的文件系统路径。
    /// 参数：name - 图标名称（如 "tray_icon", "web_pages" 等）。
    /// 返回：图标路径，未注册则返回 None。
    pub fn get_app_icon_path(&self, name: &str) -> Option<String> {
        self.app_resource.get_icon_path(name)
    }

    // ===== 快捷方式解析 =====

    /// 解析 .lnk 快捷方式文件的目标路径。
    /// 参数：lnk_path - .lnk 文件的路径。
    /// 返回：解析成功返回目标路径，失败返回 None。
    pub fn resolve_lnk_target(&self, lnk_path: &str) -> Option<String> {
        self.lnk_resolver.resolve_lnk_target(lnk_path)
    }

    /// 解析指定目录下的 desktop.ini 文件，提取 [LocalizedFileNames] 部分。
    /// 参数：dir_path - 要解析的目录路径。
    /// 返回：从原始文件名到本地化名称的映射。
    pub fn parse_localized_names_from_dir(
        &self,
        dir_path: &std::path::Path,
    ) -> std::collections::HashMap<String, String> {
        self.resource_loader
            .parse_localized_names_from_dir(dir_path)
    }

    // ===== 配置管理 =====

    /// 更新插件的 SDK 配置。
    /// 参数：config - 新的插件 SDK 配置。
    /// 返回：无。
    /// 特性：立即生效，影响后续所有服务调用。
    pub fn update_config(&self, config: PluginSdkConfig) {
        *self.config.write() = config;
    }

    // ===== 能力查询 =====

    /// 查询当前平台支持的能力集合。
    /// 参数：无。
    /// 返回：平台能力的不可变引用。
    pub fn capabilities(&self) -> &PlatformCapabilities {
        &self.capabilities
    }

    // ===== 参数解析服务 =====

    /// 解析参数模板
    ///
    /// 参数：
    /// - template: 包含占位符的模板字符串
    /// - user_args: 用户输入的参数列表
    /// - snapshot: 系统参数快照（不透明句柄）
    ///
    /// 返回：填充后的完整字符串
    pub async fn resolve_parameters(
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

    /// 统计模板中需要用户输入的参数数量
    ///
    /// 参数：template - 模板字符串
    /// 返回：位置参数的数量
    pub fn count_user_parameters(&self, template: &str) -> usize {
        self.parameter_resolver.count_user_parameters(template)
    }

    /// 检查模板是否包含系统参数
    ///
    /// 参数：template - 模板字符串
    /// 返回：是否包含系统参数
    pub fn has_system_parameters(&self, template: &str) -> bool {
        self.parameter_resolver.has_system_parameters(template)
    }

    // ===== 定时器服务 =====

    /// 创建一个一次性定时器，在指定延迟后触发回调。
    ///
    /// 参数：
    /// - delay: 触发延迟时长
    /// - callback: 触发时调用的回调函数
    ///
    /// 返回：TimerId，可用于取消定时器。
    pub async fn set_timeout(
        &self,
        delay: std::time::Duration,
        callback: TimerCallback,
    ) -> Result<TimerId, HostApiError> {
        self.timer_manager
            .set_timer(delay, TimerMode::OneShot, callback)
            .await
    }

    /// 创建一个重复定时器，每隔指定间隔触发回调。
    ///
    /// 参数：
    /// - interval: 触发间隔时长
    /// - callback: 每次触发时调用的回调函数
    ///
    /// 返回：TimerId，可用于取消定时器。
    pub async fn set_interval(
        &self,
        interval: std::time::Duration,
        callback: TimerCallback,
    ) -> Result<TimerId, HostApiError> {
        self.timer_manager
            .set_timer(interval, TimerMode::Interval, callback)
            .await
    }

    /// 取消指定 ID 的定时器。
    ///
    /// 参数：id - 要取消的定时器 ID。
    pub async fn cancel_timer(&self, id: TimerId) -> Result<(), HostApiError> {
        self.timer_manager.cancel_timer(id).await
    }

    /// 取消所有定时器。
    pub async fn cancel_all_timers(&self) -> Result<(), HostApiError> {
        self.timer_manager.cancel_all().await
    }

    // ===== 资源管理 =====

    /// 上传资源文件到本插件的资源空间。
    pub async fn resource_upload(
        &self,
        purpose: &str,
        file_path: &str,
        max_size: Option<u64>,
    ) -> Result<String, HostApiError> {
        let path = std::path::Path::new(file_path);
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("bin")
            .to_lowercase();

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

        let hash = short_hash(&data);
        let filename = format!("{}_{}.{}", purpose, hash, ext);
        let storage_path = build_resource_path(&self.plugin_id, Some(&filename));
        self.storage
            .upload(&storage_path, &data)
            .await
            .map_err(|e| HostApiError::StorageOperationFailed {
                file: storage_path,
                reason: e.to_string(),
            })?;
        Ok(format!("res://{}", filename))
    }

    /// 获取资源文件内容。
    pub async fn resource_get(&self, resource_id: &str) -> Result<Vec<u8>, HostApiError> {
        let path = build_resource_path(&self.plugin_id, Some(resource_id));
        self.storage
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

    /// 删除资源文件。
    pub async fn resource_delete(&self, resource_id: &str) -> Result<(), HostApiError> {
        let path = build_resource_path(&self.plugin_id, Some(resource_id));
        self.storage
            .delete(&path)
            .await
            .map_err(|e| HostApiError::StorageOperationFailed {
                file: path,
                reason: e.to_string(),
            })
    }

    /// 列出本插件的所有资源。
    pub async fn resource_list(&self) -> Result<Vec<String>, HostApiError> {
        let prefix = build_resource_path(&self.plugin_id, None);
        self.storage
            .list(&prefix)
            .await
            .map_err(|e| HostApiError::StorageOperationFailed {
                file: prefix,
                reason: e.to_string(),
            })
    }

    // ===== 推送式回调注册 =====

    /// 为回调 ID 添加插件前缀，避免不同插件间的 ID 冲突。
    fn prefix_callback_id(&self, id: &str) -> String {
        format!("{}:{}", self.plugin_id, id)
    }

    /// 注册按键事件回调。
    /// 参数：id - 回调标识（自动前缀化为 "{plugin_id}:{id}"）；filter - 事件过滤器；callback - 回调函数。
    pub fn register_hotkey_callback(
        &self,
        id: &str,
        filter: HotkeyEventFilter,
        callback: HotkeyCallback,
    ) {
        let prefixed = self.prefix_callback_id(id);
        self.hotkey_manager
            .register_callback(&prefixed, filter, callback);
    }

    /// 注销按键事件回调。
    /// 参数：id - 回调标识（自动前缀化为 "{plugin_id}:{id}"）。
    pub fn unregister_hotkey_callback(&self, id: &str) {
        let prefixed = self.prefix_callback_id(id);
        self.hotkey_manager.unregister_callback(&prefixed);
    }

    /// 注册安装事件回调。
    /// 参数：id - 回调标识（自动前缀化为 "{plugin_id}:{id}"）；callback - 回调函数。
    pub fn register_installation_callback(&self, id: &str, callback: InstallationCallback) {
        let prefixed = self.prefix_callback_id(id);
        self.installation_monitor
            .register_callback(&prefixed, callback);
    }

    /// 注销安装事件回调。
    /// 参数：id - 回调标识（自动前缀化为 "{plugin_id}:{id}"）。
    pub fn unregister_installation_callback(&self, id: &str) {
        let prefixed = self.prefix_callback_id(id);
        self.installation_monitor.unregister_callback(&prefixed);
    }

    /// 注册焦点事件回调。
    /// 参数：id - 回调标识（自动前缀化为 "{plugin_id}:{id}"）；callback - 回调函数。
    pub fn register_focus_callback(&self, id: &str, callback: FocusCallback) {
        let prefixed = self.prefix_callback_id(id);
        self.focus_monitor.register_callback(&prefixed, callback);
    }

    /// 注销焦点事件回调。
    /// 参数：id - 回调标识（自动前缀化为 "{plugin_id}:{id}"）。
    pub fn unregister_focus_callback(&self, id: &str) {
        let prefixed = self.prefix_callback_id(id);
        self.focus_monitor.unregister_callback(&prefixed);
    }
}

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
    storage: RwLock<Arc<dyn StorageService>>,
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
        let handle = Arc::new(PluginHandle {
            plugin_id: plugin_id.to_string(),
            config: RwLock::new(config),
            capabilities: self.capabilities.clone(),
            icon_extractor: self.icon_extractor.clone(),
            icon_cache: self.icon_cache.clone(),
            shell_executor: self.shell_executor.clone(),
            window_manager: self.window_manager.clone(),
            path_resolver: self.path_resolver.clone(),
            app_enumerator: self.app_enumerator.clone(),
            app_launcher: self.app_launcher.clone(),
            lnk_resolver: self.lnk_resolver.clone(),
            resource_loader: self.resource_loader.clone(),
            parameter_resolver: self.parameter_resolver.clone(),
            timer_manager: self.timer_manager.clone(),
            app_resource: self.app_resource.clone(),
            storage: self.storage(),
            hotkey_manager: self.hotkey_manager.clone(),
            installation_monitor: self.installation_monitor.clone(),
            focus_monitor: self.focus_monitor.clone(),
        });
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
        self.storage.read().clone()
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
}

impl HostApiBuilder {
    /// 创建 HostApiBuilder 实例。
    /// 参数：icon_cache_dir - 图标缓存目录。
    /// 返回：HostApiBuilder 实例。
    fn new(icon_cache_dir: String) -> Self {
        Self {
            icon_cache_dir,
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
        }
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

    /// 构建 HostApi 实例。
    /// 参数：无。
    /// 返回：构建完成的 HostApi 实例，如果缺少必需组件则 panic。
    #[cfg(target_os = "windows")]
    pub fn build(self) -> HostApi {
        let icon_cache = Arc::new(IconCacheService::new(self.icon_cache_dir));
        icon_cache.init();
        HostApi {
            handles: DashMap::new(),
            capabilities: PlatformCapabilities::windows(),
            icon_cache,
            icon_extractor: self.icon_extractor.expect("missing icon_extractor"),
            shell_executor: self.shell_executor.expect("missing shell_executor"),
            window_manager: self.window_manager.expect("missing window_manager"),
            path_resolver: self.path_resolver.expect("missing path_resolver"),
            app_enumerator: self.app_enumerator.expect("missing app_enumerator"),
            app_launcher: self.app_launcher.expect("missing app_launcher"),
            lnk_resolver: self.lnk_resolver.expect("missing lnk_resolver"),
            resource_loader: self.resource_loader.expect("missing resource_loader"),
            parameter_resolver: self.parameter_resolver.expect("missing parameter_resolver"),
            clipboard_provider: self.clipboard_provider.expect("missing clipboard_provider"),
            window_handle_provider: self
                .window_handle_provider
                .expect("missing window_handle_provider"),
            selection_provider: self.selection_provider.expect("missing selection_provider"),
            autostart_manager: self.autostart_manager.expect("missing autostart_manager"),
            hotkey_manager: self.hotkey_manager.expect("missing hotkey_manager"),
            installation_monitor: self
                .installation_monitor
                .expect("missing installation_monitor"),
            timer_manager: self.timer_manager.expect("missing timer_manager"),
            storage: RwLock::new(self.storage_service.expect("missing storage_service")),
            app_resource: self.app_resource.expect("missing app_resource"),
            focus_monitor: self.focus_monitor.expect("missing focus_monitor"),
            notify_callback: RwLock::new(self.notify_callback.expect("missing notify_callback")),
            hide_window_callback: RwLock::new(
                self.hide_window_callback
                    .expect("missing hide_window_callback"),
            ),
            show_window_callback: RwLock::new(
                self.show_window_callback
                    .expect("missing show_window_callback"),
            ),
            is_window_visible_callback: RwLock::new(
                self.is_window_visible_callback
                    .expect("missing is_window_visible_callback"),
            ),
        }
    }
}

/// 构建资源存储路径。
/// 使用 PathBuf 确保路径构建的安全性，避免路径遍历攻击。
/// 返回 Unix 风格路径（存储后端约定）。
fn build_resource_path(plugin_id: &str, filename: Option<&str>) -> String {
    let mut path = std::path::PathBuf::new();
    path.push("resources");
    path.push(plugin_id);
    if let Some(name) = filename {
        path.push(name);
    }
    // 统一使用 Unix 风格路径分隔符
    path.to_string_lossy().replace('\\', "/")
}

/// 生成数据的短哈希（用于资源文件名去重）。
fn short_hash(data: &[u8]) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(data);
    let hash = hasher.finalize();
    // 取前 12 个 hex 字符，足够避免碰撞
    let hex = hash.to_hex();
    hex[..12].to_string()
}
