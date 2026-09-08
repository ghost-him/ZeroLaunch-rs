use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use parking_lot::RwLock;

use crate::host::plugin_host::PluginHost;
use crate::host::{CacheLevel, HostApiError, OpenTarget, PluginSdkConfig};
use crate::platform::capabilities::PlatformCapabilities;
use crate::services::app::AppInfo;
use crate::services::focus_monitor::FocusCallback;
use crate::services::hotkey::types::{HotkeyCallback, HotkeyEventFilter};
use crate::services::installation_monitor::types::InstallationCallback;
use crate::services::model::{
    ModelChatRequest, ModelChatResponse, ModelEmbeddingRequest, ModelEmbeddingResponse, ModelError,
    ModelInfo, ModelSimilarityRequest, ModelSimilarityResponse,
};
use crate::services::parameter::types::ParameterSnapshot;
use crate::services::path::path_resolver::KnownPath;
use crate::services::theme::Theme;
use crate::services::timer::types::{TimerCallback, TimerId};
use crate::services::IconRequest;

/// 插件服务句柄（插件接口层）。
/// 绑定插件身份（plugin_id）、配置（PluginSdkConfig）与能力集，并持有宿主操作契约
/// `Arc<dyn PluginHost>`；所有服务操作委托宿主执行，宿主内部再决定直接处理或
/// 转发平台实现。插件通过 HostApi::register() 获取此句柄。
pub struct PluginHandle {
    plugin_id: String,
    config: RwLock<PluginSdkConfig>,
    capabilities: PlatformCapabilities,
    /// 宿主操作契约实现（Arc 共享，所有委托操作的执行节点）。
    host: Arc<dyn PluginHost>,
}

impl PluginHandle {
    /// 绑定插件身份与宿主契约构造句柄。
    /// 参数：plugin_id - 插件唯一标识；config - 插件 SDK 配置；
    ///       capabilities - 平台能力集；host - 宿主操作契约实现。
    pub fn new(
        plugin_id: String,
        config: PluginSdkConfig,
        capabilities: PlatformCapabilities,
        host: Arc<dyn PluginHost>,
    ) -> Self {
        Self {
            plugin_id,
            config: RwLock::new(config),
            capabilities,
            host,
        }
    }

    /// 返回当前插件 ID（宿主模型缓存等按插件隔离目录需要）。
    pub fn plugin_id(&self) -> &str {
        &self.plugin_id
    }

    /// 获取当前图标缓存等级，None 时返回默认值 Full。
    fn icon_cache_level(&self) -> CacheLevel {
        self.config.read().icon_cache_level.unwrap_or_default()
    }

    /// 更新插件的 SDK 配置，立即生效，影响后续所有服务调用。
    pub fn update_config(&self, config: PluginSdkConfig) {
        *self.config.write() = config;
    }

    /// 查询当前平台支持的能力集合。
    pub fn capabilities(&self) -> &PlatformCapabilities {
        &self.capabilities
    }

    // ===== 图标服务（委托宿主） =====

    /// 根据图标请求提取图标数据，行为由注册时的缓存等级决定。
    pub async fn get_icon(&self, request: IconRequest) -> Result<Vec<u8>, HostApiError> {
        let level = self.icon_cache_level();
        self.host.get_icon(&request, level).await
    }

    /// 提取图标数据，失败时回退到默认图标（永不返回错误）。
    pub async fn get_icon_or_default(&self, request: IconRequest) -> Vec<u8> {
        let level = self.icon_cache_level();
        self.host.get_icon_or_default(&request, level).await
    }

    /// 强制从磁盘提取图标数据并根据缓存等级更新缓存。
    pub async fn get_icon_and_update_cache(
        &self,
        request: IconRequest,
    ) -> Result<Vec<u8>, HostApiError> {
        let level = self.icon_cache_level();
        self.host.get_icon_and_update_cache(&request, level).await
    }

    /// 覆盖指定 IconRequest 的缓存图标为自定义图标文件。
    pub async fn override_icon_cache(
        &self,
        original_request: &IconRequest,
        custom_icon_path: &str,
    ) -> Result<(), HostApiError> {
        self.host
            .override_icon_cache(original_request, custom_icon_path)
            .await
    }

    // ===== Shell 服务（委托宿主） =====

    /// 使用系统默认方式打开目标（文件/网址/文件夹）。
    pub async fn shell_open(&self, target: OpenTarget) -> Result<(), HostApiError> {
        self.host.shell_open(target).await
    }

    /// 在文件资源管理器中打开指定路径的父目录并选中该文件。
    pub async fn shell_open_folder(&self, path: &str) -> Result<(), HostApiError> {
        self.host.shell_open_folder(path).await
    }

    /// 以管理员权限启动程序。
    pub async fn shell_execute_elevation(&self, path: &str) -> Result<(), HostApiError> {
        self.host.shell_execute_elevation(path).await
    }

    /// 执行命令字符串（后台运行，无窗口）。
    pub async fn shell_execute_command(&self, command: &str) -> Result<(), HostApiError> {
        self.host.shell_execute_command(command).await
    }

    // ===== 窗口服务（委托宿主） =====

    /// 根据进程名（如 "chrome.exe"）激活已存在的窗口。
    pub async fn activate_window_by_process(
        &self,
        process_name: &str,
    ) -> Result<bool, HostApiError> {
        self.host.activate_window_by_process(process_name).await
    }

    /// 根据窗口标题的部分内容激活已存在的窗口。
    pub async fn activate_window_by_title(&self, title: &str) -> Result<bool, HostApiError> {
        self.host.activate_window_by_title(title).await
    }

    /// 根据进程 PID 激活已存在的窗口。
    pub async fn activate_window_by_pid(&self, pid: u32) -> Result<bool, HostApiError> {
        self.host.activate_window_by_pid(pid).await
    }

    // ===== 路径服务（委托宿主） =====

    /// 根据已知路径类型解析实际文件系统路径。
    pub fn resolve_path(&self, path: KnownPath) -> Result<String, HostApiError> {
        self.host.resolve_path(path)
    }

    // ===== 剪贴板服务（委托宿主） =====

    /// 将文本写入系统剪贴板。
    pub fn set_clipboard_text(&self, text: &str) -> Result<(), HostApiError> {
        self.host.set_clipboard_text(text)
    }

    // ===== 应用服务（委托宿主） =====

    /// 枚举当前平台已安装的应用。
    pub async fn enumerate_apps(&self) -> Vec<AppInfo> {
        self.host.enumerate_apps().await
    }

    /// 启动指定应用。
    pub async fn launch_app(
        &self,
        app_id: &str,
        args: Option<&[String]>,
    ) -> Result<u32, HostApiError> {
        self.host.launch_app(app_id, args).await
    }

    // ===== 应用资源服务（委托宿主） =====

    /// 根据名称获取内置图标资源的文件系统路径。
    pub fn get_app_icon_path(&self, name: &str) -> Option<String> {
        self.host.get_app_icon_path(name)
    }

    // ===== 快捷方式解析（委托宿主） =====

    /// 解析 .lnk 快捷方式文件的目标路径。
    pub fn resolve_lnk_target(&self, lnk_path: &str) -> Option<String> {
        self.host.resolve_lnk_target(lnk_path)
    }

    /// 解析指定目录下的 desktop.ini 文件，提取 [LocalizedFileNames] 部分。
    pub fn parse_localized_names_from_dir(&self, dir_path: &Path) -> HashMap<String, String> {
        self.host.parse_localized_names_from_dir(dir_path)
    }

    // ===== 主题服务（委托宿主） =====

    /// 查询宿主当前实际生效的界面主题（system 模式由宿主解析）。
    pub fn get_theme(&self) -> Result<Theme, HostApiError> {
        self.host.get_theme()
    }

    /// 查询系统主题（未应用宿主显式 light/dark 配置）。
    pub fn get_system_theme(&self) -> Result<Theme, HostApiError> {
        self.host.get_system_theme()
    }

    // ===== 模型服务（委托宿主） =====

    /// 全网模型清单（聚合缓存，含所有已注册提供方）。
    pub fn model_list(&self) -> Vec<ModelInfo> {
        self.host.model_list()
    }

    /// 按 model_id 调用文本生成。
    pub async fn model_chat(&self, req: ModelChatRequest) -> Result<ModelChatResponse, ModelError> {
        self.host.model_chat(req).await
    }

    /// 按 model_id 调用文本向量化（task_type 必填）。
    pub async fn model_embedding(
        &self,
        req: ModelEmbeddingRequest,
    ) -> Result<ModelEmbeddingResponse, ModelError> {
        self.host.model_embedding(req).await
    }

    /// 按 model_id 计算查询向量与多个目标向量的相似度。
    pub async fn model_similarity(
        &self,
        req: ModelSimilarityRequest,
    ) -> Result<ModelSimilarityResponse, ModelError> {
        self.host.model_similarity(req).await
    }

    // ===== 参数解析服务（委托宿主） =====

    /// 解析参数模板。
    pub async fn resolve_parameters(
        &self,
        template: &str,
        user_args: &[String],
        snapshot: &ParameterSnapshot,
    ) -> Result<String, HostApiError> {
        self.host
            .resolve_parameters(template, user_args, snapshot)
            .await
    }

    /// 统计模板中需要用户输入的参数数量。
    pub fn count_user_parameters(&self, template: &str) -> usize {
        self.host.count_user_parameters(template)
    }

    /// 检查模板是否包含系统参数。
    pub fn has_system_parameters(&self, template: &str) -> bool {
        self.host.has_system_parameters(template)
    }

    // ===== 定时器服务（委托宿主） =====

    /// 创建一个一次性定时器，在指定延迟后触发回调。
    pub async fn set_timeout(
        &self,
        delay: Duration,
        callback: TimerCallback,
    ) -> Result<TimerId, HostApiError> {
        self.host.set_timeout(delay, callback).await
    }

    /// 创建一个重复定时器，每隔指定间隔触发回调。
    pub async fn set_interval(
        &self,
        interval: Duration,
        callback: TimerCallback,
    ) -> Result<TimerId, HostApiError> {
        self.host.set_interval(interval, callback).await
    }

    /// 取消指定 ID 的定时器。
    pub async fn cancel_timer(&self, id: TimerId) -> Result<(), HostApiError> {
        self.host.cancel_timer(id).await
    }

    /// 取消所有定时器。
    pub async fn cancel_all_timers(&self) -> Result<(), HostApiError> {
        self.host.cancel_all_timers().await
    }

    // ===== 资源管理（委托宿主，插件作用域） =====

    /// 上传资源文件到本插件的资源空间。
    pub async fn resource_upload(
        &self,
        resource_id: &str,
        file_path: &str,
        max_size: Option<u64>,
    ) -> Result<String, HostApiError> {
        self.host
            .resource_upload(&self.plugin_id, resource_id, file_path, max_size)
            .await
    }

    /// 直接写入资源字节数据，无需先创建临时文件或提供本地路径。
    pub async fn resource_put(&self, resource_id: &str, data: &[u8]) -> Result<(), HostApiError> {
        self.host
            .resource_put(&self.plugin_id, resource_id, data)
            .await
    }

    /// 获取资源文件内容。
    pub async fn resource_get(&self, resource_id: &str) -> Result<Vec<u8>, HostApiError> {
        self.host.resource_get(&self.plugin_id, resource_id).await
    }

    /// 删除资源文件。
    pub async fn resource_delete(&self, resource_id: &str) -> Result<(), HostApiError> {
        self.host
            .resource_delete(&self.plugin_id, resource_id)
            .await
    }

    /// 列出本插件的所有资源。
    pub async fn resource_list(&self) -> Result<Vec<String>, HostApiError> {
        self.host.resource_list(&self.plugin_id).await
    }

    // ===== 本地缓存（委托宿主，插件作用域） =====

    /// 写入插件本地缓存。
    /// 与 resource_* 的区别：缓存存放可再生的本地数据（如模型向量），
    /// 不经过 StorageService，WebDAV 同步模式不会上传远端。
    pub async fn cache_put(
        &self,
        domain: &str,
        key: &str,
        data: &[u8],
    ) -> Result<(), HostApiError> {
        self.host
            .cache_put(&self.plugin_id, domain, key, data)
            .await
    }

    /// 读取插件本地缓存；缓存不存在时返回 Ok(None)。
    pub async fn cache_get(
        &self,
        domain: &str,
        key: &str,
    ) -> Result<Option<Vec<u8>>, HostApiError> {
        self.host.cache_get(&self.plugin_id, domain, key).await
    }

    /// 删除插件本地缓存条目；缓存不存在时视为成功。
    pub async fn cache_delete(&self, domain: &str, key: &str) -> Result<(), HostApiError> {
        self.host.cache_delete(&self.plugin_id, domain, key).await
    }

    /// 容量控制：缓存域条目超过 max_entries 时按修改时间删除最旧条目。
    pub async fn cache_cleanup(
        &self,
        domain: &str,
        max_entries: usize,
    ) -> Result<(), HostApiError> {
        self.host
            .cache_cleanup(&self.plugin_id, domain, max_entries)
            .await
    }

    // ===== 推送式回调注册（委托宿主，ID 由宿主按插件前缀化） =====

    /// 注册按键事件回调。ID 自动前缀化为 "{plugin_id}:{id}"。
    pub fn register_hotkey_callback(
        &self,
        id: &str,
        filter: HotkeyEventFilter,
        callback: HotkeyCallback,
    ) {
        self.host
            .register_hotkey_callback(&self.plugin_id, id, filter, callback);
    }

    /// 注销按键事件回调。
    pub fn unregister_hotkey_callback(&self, id: &str) {
        self.host.unregister_hotkey_callback(&self.plugin_id, id);
    }

    /// 注册安装事件回调。ID 自动前缀化为 "{plugin_id}:{id}"。
    pub fn register_installation_callback(&self, id: &str, callback: InstallationCallback) {
        self.host
            .register_installation_callback(&self.plugin_id, id, callback);
    }

    /// 注销安装事件回调。
    pub fn unregister_installation_callback(&self, id: &str) {
        self.host
            .unregister_installation_callback(&self.plugin_id, id);
    }

    /// 注册焦点事件回调。ID 自动前缀化为 "{plugin_id}:{id}"。
    pub fn register_focus_callback(&self, id: &str, callback: FocusCallback) {
        self.host
            .register_focus_callback(&self.plugin_id, id, callback);
    }

    /// 注销焦点事件回调。
    pub fn unregister_focus_callback(&self, id: &str) {
        self.host.unregister_focus_callback(&self.plugin_id, id);
    }
}

/// 构建资源存储路径，校验文件名防止路径遍历攻击。
/// 使用 PathBuf 确保路径构建的安全性。
/// 返回 Unix 风格路径（存储后端约定）。
/// 供宿主实现 PluginHost 资源操作时使用。
pub fn build_resource_path(
    plugin_id: &str,
    filename: Option<&str>,
) -> Result<String, HostApiError> {
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

/// 构建本地缓存文件路径：`<cache_root>/<plugin_id>/<domain>/<key>`。
/// 校验 domain 与 key（拒绝空串、"."、".." 与路径穿越），策略同 build_resource_path。
/// 缓存根目录为宿主 app data 下的 plugin-cache/，不经 StorageService。
/// 供宿主实现 PluginHost 缓存操作时使用。
pub fn build_cache_path(
    cache_root: &str,
    plugin_id: &str,
    domain: &str,
    key: &str,
) -> Result<String, HostApiError> {
    for segment in [domain, key] {
        if segment.is_empty() || segment == "." || segment == ".." {
            return Err(HostApiError::PathTraversalRejected {
                path: segment.to_string(),
            });
        }
    }
    let base = std::path::Path::new(cache_root)
        .join(plugin_id)
        .join(domain);
    let base_normalized = normalize_path(&base);
    let mut path = base;
    path.push(key);
    let normalized = normalize_path(&path);
    if normalized != base_normalized && !normalized.starts_with(&base_normalized) {
        return Err(HostApiError::PathTraversalRejected {
            path: key.to_string(),
        });
    }
    Ok(path.to_string_lossy().replace('\\', "/"))
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
    fn build_cache_path_accepts_valid_segments() {
        let result = build_cache_path(
            "C:/mock/zl-cache",
            "test-plugin",
            "model-embedding",
            "ab/abc123.bin",
        );
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.starts_with("C:/mock/zl-cache/test-plugin/model-embedding/"));
        assert!(path.ends_with("abc123.bin"));
    }

    #[test]
    fn build_cache_path_rejects_traversal() {
        for (domain, key) in [
            ("..", "a.bin"),
            ("a", "../../x.bin"),
            ("a", "../b/x.bin"),
            ("a", ".."),
            ("a", "."),
            ("a", ""),
        ] {
            assert!(
                build_cache_path("C:/mock/zl-cache", "test-plugin", domain, key).is_err(),
                "应拒绝 domain={domain:?} key={key:?}"
            );
        }
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
