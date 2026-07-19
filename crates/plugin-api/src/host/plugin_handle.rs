use crate::host::{CacheLevel, HostApiError, OpenTarget};
use crate::platform::capabilities::PlatformCapabilities;
use crate::services::app::{AppEnumerator, AppInfo, AppLauncher};
use crate::services::focus_monitor::{FocusCallback, FocusMonitor};
use crate::services::hotkey::types::{HotkeyCallback, HotkeyEventFilter};
use crate::services::hotkey::HotkeyManager;
use crate::services::icon::icon_cache::IconCacheService;
use crate::services::icon::icon_extractor::IconExtractor;
use crate::services::installation_monitor::types::InstallationCallback;
use crate::services::installation_monitor::InstallationMonitor;
use crate::services::parameter::resolver::ParameterResolver;
use crate::services::parameter::types::ParameterSnapshot;
use crate::services::path::path_resolver::{KnownPath, PathResolver};
use crate::services::resource::AppResourceService;
use crate::services::shell::lnk_resolver::LnkResolver;
use crate::services::shell::resource_loader::ResourceLoader;
use crate::services::shell::ShellExecutor;
use crate::services::storage::storage_service::StorageService;
use crate::services::timer::types::{TimerCallback, TimerId, TimerMode};
use crate::services::timer::TimerManager;
use crate::services::window::WindowManager;
use crate::services::IconRequest;
use parking_lot::RwLock;
use std::sync::Arc;

use super::sdk_config::PluginSdkConfig;
/// 插件服务句柄，绑定插件身份与配置。
/// 跨平台 struct，通过 Arc<dyn IconExtractor> 等平台 trait 注入平台代码。
/// 插件通过 HostApi::register() 获取此句柄，后续所有服务调用通过句柄完成。
/// 句柄自动应用注册时的插件配置（如缓存等级），插件无需在每次调用时传递配置。
pub struct PluginHandle {
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
    /// 存储服务，由 HostApi 注入（共享 RwLock，reconfigure 后自动可见）
    storage: Arc<RwLock<Arc<dyn StorageService>>>,
    /// 按键管理器，由 HostApi 注入
    hotkey_manager: Arc<dyn HotkeyManager>,
    /// 安装监控器，由 HostApi 注入
    installation_monitor: Arc<dyn InstallationMonitor>,
    /// 聚焦监控器，由 HostApi 注入
    focus_monitor: Arc<dyn FocusMonitor>,
}

impl PluginHandle {
    /// Creates a new PluginHandle with all the service references injected.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        plugin_id: String,
        config: PluginSdkConfig,
        capabilities: PlatformCapabilities,
        icon_extractor: Arc<dyn IconExtractor>,
        icon_cache: Arc<IconCacheService>,
        shell_executor: Arc<dyn ShellExecutor>,
        window_manager: Arc<dyn WindowManager>,
        path_resolver: Arc<dyn PathResolver>,
        app_enumerator: Arc<dyn AppEnumerator>,
        app_launcher: Arc<dyn AppLauncher>,
        lnk_resolver: Arc<dyn LnkResolver>,
        resource_loader: Arc<dyn ResourceLoader>,
        parameter_resolver: Arc<dyn ParameterResolver>,
        timer_manager: Arc<dyn TimerManager>,
        app_resource: Arc<AppResourceService>,
        storage: Arc<RwLock<Arc<dyn StorageService>>>,
        hotkey_manager: Arc<dyn HotkeyManager>,
        installation_monitor: Arc<dyn InstallationMonitor>,
        focus_monitor: Arc<dyn FocusMonitor>,
    ) -> Self {
        Self {
            plugin_id,
            config: RwLock::new(config),
            capabilities,
            icon_extractor,
            icon_cache,
            shell_executor,
            window_manager,
            path_resolver,
            app_enumerator,
            app_launcher,
            lnk_resolver,
            resource_loader,
            parameter_resolver,
            timer_manager,
            app_resource,
            storage,
            hotkey_manager,
            installation_monitor,
            focus_monitor,
        }
    }

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

    /// 覆盖指定 IconRequest 的缓存图标为自定义图标文件。
    /// 参数：original_request - 需要覆盖图标的原始 IconRequest
    ///       custom_icon_path - 用户选择的自定义图标文件路径
    /// 返回：成功返回 Ok(()), 失败返回 HostApiError
    pub async fn override_icon_cache(
        &self,
        original_request: &IconRequest,
        custom_icon_path: &str,
    ) -> Result<(), HostApiError> {
        let hash_key = original_request.get_hash_string() + ".png";

        // 从自定义文件提取并处理图标
        let custom_request = IconRequest::Path(custom_icon_path.to_string());
        let data = self
            .icon_extractor
            .extract_and_process(&custom_request)
            .await?;

        // 覆盖写入 L1 + L2 缓存
        self.icon_cache.set_l1(&hash_key, data.clone());
        self.icon_cache.set_l2(&hash_key, data).await;

        Ok(())
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
        let storage_path = build_resource_path(&self.plugin_id, Some(resource_id))?;
        let storage = self.storage.read().clone();
        storage.upload(&storage_path, &data).await.map_err(|e| {
            HostApiError::StorageOperationFailed {
                file: storage_path,
                reason: e.to_string(),
            }
        })?;
        Ok(resource_id.to_string())
    }

    /// 直接写入资源字节数据，无需先创建临时文件或提供本地路径。
    /// 参数：resource_id - 资源标识符；data - 资源字节内容。
    /// 返回：成功返回 Ok(())，失败返回 HostApiError。
    pub async fn resource_put(&self, resource_id: &str, data: &[u8]) -> Result<(), HostApiError> {
        let storage_path = build_resource_path(&self.plugin_id, Some(resource_id))?;
        let storage: Arc<dyn StorageService> = self.storage.read().clone();
        storage.upload(&storage_path, data).await.map_err(|e| {
            HostApiError::StorageOperationFailed {
                file: storage_path,
                reason: e.to_string(),
            }
        })
    }

    /// 获取资源文件内容。
    pub async fn resource_get(&self, resource_id: &str) -> Result<Vec<u8>, HostApiError> {
        let path = build_resource_path(&self.plugin_id, Some(resource_id))?;
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

    /// 删除资源文件。
    pub async fn resource_delete(&self, resource_id: &str) -> Result<(), HostApiError> {
        let path = build_resource_path(&self.plugin_id, Some(resource_id))?;
        let storage = self.storage.read().clone();
        storage
            .delete(&path)
            .await
            .map_err(|e| HostApiError::StorageOperationFailed {
                file: path,
                reason: e.to_string(),
            })
    }

    /// 列出本插件的所有资源。
    pub async fn resource_list(&self) -> Result<Vec<String>, HostApiError> {
        let prefix = build_resource_path(&self.plugin_id, None)?;
        let storage = self.storage.read().clone();
        storage
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

/// 构建资源存储路径，校验文件名防止路径遍历攻击。
/// 使用 PathBuf 确保路径构建的安全性。
/// 返回 Unix 风格路径（存储后端约定）。
fn build_resource_path(plugin_id: &str, filename: Option<&str>) -> Result<String, HostApiError> {
    let base = std::path::PathBuf::from_iter(["resources", plugin_id]);
    let base_normalized = normalize_path(&base);

    let mut path = base.clone();
    if let Some(name) = filename {
        // 拒绝空字符串以及 "." / ".." 字面量
        if name.is_empty() || name == "." || name == ".." {
            return Err(HostApiError::PathTraversalRejected {
                path: name.to_string(),
            });
        }
        path.push(name);
        let normalized = normalize_path(&path);
        // Path::starts_with 按组件边界匹配。因此 "resources/test" 不会错误地
        // 作为 "resources/test_evil/..." 的前缀，无需追加尾部分隔符。
        let is_valid = normalized == base_normalized || normalized.starts_with(&base_normalized);
        if !is_valid {
            return Err(HostApiError::PathTraversalRejected {
                path: name.to_string(),
            });
        }
    }
    Ok(path.to_string_lossy().replace('\\', "/"))
}

/// 标准化路径，解析 `.` 和 `..` 组件。
/// 纯内存操作，不访问文件系统。
fn normalize_path(path: &std::path::Path) -> std::path::PathBuf {
    let mut result = std::path::PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                result.pop();
            }
            std::path::Component::CurDir => {
                // 跳过
            }
            other => {
                result.push(other);
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_path_removes_cur_dir() {
        let input = std::path::Path::new("a/./b/./c");
        let result = normalize_path(input);
        assert_eq!(result, std::path::PathBuf::from("a/b/c"));
    }

    #[test]
    fn normalize_path_resolves_parent_dir() {
        let input = std::path::Path::new("a/b/../c");
        let result = normalize_path(input);
        assert_eq!(result, std::path::PathBuf::from("a/c"));
    }

    #[test]
    fn normalize_path_handles_leading_dotdot() {
        let input = std::path::Path::new("../../../etc/passwd");
        let result = normalize_path(input);
        // 前导 .. pop 空栈，最终只剩下 etc/passwd
        assert_eq!(result, std::path::PathBuf::from("etc/passwd"));
    }

    #[test]
    fn build_resource_path_rejects_parent_dir_traversal() {
        let result = build_resource_path("test-plugin", Some("../../../secret"));
        assert!(result.is_err());
        match result {
            Err(HostApiError::PathTraversalRejected { path }) => {
                assert!(path.contains(".."));
            }
            _ => panic!("expected PathTraversalRejected"),
        }
    }

    #[test]
    fn build_resource_path_rejects_cross_plugin_traversal() {
        // 插件 "test" 尝试通过 .. 访问插件 "test_evil" 的资源
        let result = build_resource_path("test", Some("../test_evil/secret.txt"));
        assert!(matches!(
            result,
            Err(HostApiError::PathTraversalRejected { .. })
        ));
    }

    #[test]
    fn build_resource_path_rejects_dot_literal() {
        let result = build_resource_path("test-plugin", Some("."));
        assert!(matches!(
            result,
            Err(HostApiError::PathTraversalRejected { .. })
        ));
    }

    #[test]
    fn build_resource_path_rejects_dotdot_literal() {
        let result = build_resource_path("test-plugin", Some(".."));
        assert!(matches!(
            result,
            Err(HostApiError::PathTraversalRejected { .. })
        ));
    }

    #[test]
    fn build_resource_path_accepts_valid_filename() {
        let result = build_resource_path("test-plugin", Some("icon.png"));
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.starts_with("resources/test-plugin/"));
        assert!(path.ends_with("icon.png"));
    }

    #[test]
    fn build_resource_path_accepts_none_filename() {
        let result = build_resource_path("test-plugin", None);
        assert!(result.is_ok());
        let path = result.unwrap();
        assert_eq!(path, "resources/test-plugin");
    }

    #[test]
    fn build_resource_path_rejects_empty_filename() {
        let result = build_resource_path("test-plugin", Some(""));
        assert!(matches!(
            result,
            Err(HostApiError::PathTraversalRejected { .. })
        ));
    }

    // ── starts_with 组件级匹配验证 ────────────────────────────

    #[test]
    fn starts_with_component_boundary_prevents_false_prefix_match() {
        // 验证 Path::starts_with 按组件边界匹配：
        // "resources/test" 不是 "resources/test_evil/..." 的前缀。
        // 这意味着 build_resource_path 不需要尾部分隔符来防止误匹配。
        let base = std::path::Path::new("resources/test");
        let evil = std::path::Path::new("resources/test_evil/secret.txt");
        assert!(!evil.starts_with(base));
    }

    #[test]
    fn build_resource_path_rejects_same_prefix_traversal() {
        // 插件 "test" 尝试写 path = "test_evil/secret.txt"，该路径落在
        // resources/test/test_evil/secret.txt → 应被允许（在自己的空间内）。
        // 但尝试通过 ../ 逃逸到 test_evil 才被拒绝。
        let result = build_resource_path("test", Some("../test_evil/secret.txt"));
        assert!(matches!(
            result,
            Err(HostApiError::PathTraversalRejected { .. })
        ));
    }

    #[test]
    fn build_resource_path_allows_subdirectory_with_same_prefix() {
        // 资源名 "test_data.txt" 在插件 "test" 下 → resources/test/test_data.txt
        // 这不是路径遍历，应被允许。
        let result = build_resource_path("test", Some("test_data.txt"));
        assert!(result.is_ok());
        let path = result.unwrap();
        assert_eq!(path, "resources/test/test_data.txt");
    }

    #[test]
    fn build_resource_path_allows_nested_subdir() {
        // 允许 plugin_id/test/subdir/file.png 这种深层嵌套
        let result = build_resource_path("test", Some("subdir/file.png"));
        assert!(result.is_ok());
        let path = result.unwrap();
        assert_eq!(path, "resources/test/subdir/file.png");
    }

    #[test]
    fn pathbuf_push_empty_is_functionally_noop() {
        // 验证 PathBuf::push("") 在 Eq 和 starts_with 语义上是空操作。
        // 这确认了 build_resource_path 不需要它来提高安全性。
        let mut with_trailing = std::path::PathBuf::from("resources/test");
        with_trailing.push("");
        let without_trailing = std::path::PathBuf::from("resources/test");

        // Eq: 认为相等
        assert_eq!(with_trailing, without_trailing);

        // starts_with: 行为一致
        let child = std::path::Path::new("resources/test/icon.png");
        assert!(child.starts_with(&with_trailing));
        assert!(child.starts_with(&without_trailing));

        let unrelated = std::path::Path::new("resources/test_evil/secret.txt");
        assert!(!unrelated.starts_with(&with_trailing));
        assert!(!unrelated.starts_with(&without_trailing));
    }
}
