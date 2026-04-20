use crate::sdk::app::app_enumerator::AppEnumerator;
use crate::sdk::app::app_launcher::AppLauncher;
use crate::sdk::app::AppInfo;
use crate::sdk::icon::icon_cache::IconCacheService;
use crate::sdk::icon::icon_extractor::IconExtractor;
use crate::sdk::parameter::provider::SystemParameterProvider;
use crate::sdk::parameter::resolver::ParameterResolver;
use crate::sdk::parameter::types::ParameterSnapshot;
use crate::sdk::path::path_resolver::{KnownPath, PathResolver};
use crate::sdk::platform::capabilities::PlatformCapabilities;
use crate::sdk::shell::lnk_resolver::LnkResolver;
use crate::sdk::shell::resource_loader::ResourceLoader;
use crate::sdk::shell::ShellExecutor;
use crate::sdk::window::WindowManager;
use bincode::Encode;
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// 图标请求类型，表示不同来源的图标提取需求。
/// 各类型使用各自的提取逻辑完成图标提取。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Encode)]
pub enum IconRequest {
    /// 本地文件路径 (exe, lnk, url, ico, png) -> 提取文件图标
    Path(String),
    /// 网址 -> 下载或查找本地域名图标库
    Url(String),
    /// 文件扩展名 (.txt, .doc) -> 获取系统关联图标
    Extension(String),
}

impl IconRequest {
    /// 计算图标请求的 blake3 哈希值，用作缓存键。
    /// 参数：无。
    /// 返回：十六进制格式的哈希字符串。
    pub fn get_hash_string(&self) -> String {
        let mut hasher = blake3::Hasher::new();
        let _ = bincode::encode_into_std_write(self, &mut hasher, bincode::config::standard());
        hasher.finalize().to_hex().to_string()
    }
}

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

    // ===== 配置管理 =====

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
        }
    }
}
