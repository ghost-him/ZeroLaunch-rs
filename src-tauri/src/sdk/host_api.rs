use crate::sdk::platform::capabilities::PlatformCapabilities;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// 图标请求类型，表示不同来源的图标提取需求。
/// 各类型使用各自的提取逻辑完成图标提取。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IconRequest {
    /// 本地文件路径 (exe, lnk, url, ico, png) -> 提取文件图标
    Path(String),
    /// 网址 -> 下载或查找本地域名图标库
    Url(String),
    /// 文件扩展名 (.txt, .doc) -> 获取系统关联图标
    Extension(String),
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
}

/// 插件服务句柄，绑定插件身份与配置。
/// 插件通过 HostApi::register() 获取此句柄，后续所有服务调用通过句柄完成。
/// 句柄自动应用注册时的插件配置（如缓存等级），插件无需在每次调用时传递配置。
#[async_trait]
pub trait PluginHandle: Send + Sync {
    // ===== 图标服务 =====

    /// 根据图标请求提取图标数据，行为由注册时的缓存等级决定。
    /// 参数：request - 图标请求（路径/网址/扩展名）。
    /// 返回：PNG 格式的图标字节数据，失败返回 HostApiError。
    async fn get_icon(&self, request: IconRequest) -> Result<Vec<u8>, HostApiError>;

    /// 根据原始图标请求提取图标数据，直接从硬盘中读并根据缓存等级更新缓存信息
    /// 参数：request - 图标请求（路径/网址/扩展名）。
    /// 返回：PNG 格式的图标字节数据，失败返回 HostApiError
    async fn get_icon_and_update_cache(
        &self,
        request: IconRequest,
    ) -> Result<Vec<u8>, HostApiError>;

    // ===== Shell 服务 =====

    /// 使用系统默认方式打开目标（文件/网址/文件夹）。
    /// 参数：target - 打开目标。
    /// 返回：成功返回 Ok(())，失败返回 HostApiError。
    async fn shell_open(&self, target: OpenTarget) -> Result<(), HostApiError>;

    /// 在文件资源管理器中打开指定路径的父目录并选中该文件。
    /// 参数：path - 要打开所在位置的文件路径。
    /// 返回：成功返回 Ok(())，失败返回 HostApiError。
    async fn shell_open_folder(&self, path: &str) -> Result<(), HostApiError>;

    /// 获取系统默认浏览器名称。
    /// 参数：无。
    /// 返回：浏览器名称字符串，失败返回 HostApiError。
    async fn get_default_browser(&self) -> Result<String, HostApiError>;

    // ===== 窗口服务 =====

    /// 根据进程名（如 "chrome.exe"）激活已存在的窗口。
    /// 参数：process_name - 进程名（含扩展名）。
    /// 返回：成功激活返回 Ok(true)，未找到窗口返回 Ok(false)，失败返回 HostApiError。
    async fn activate_window_by_process(&self, process_name: &str) -> Result<bool, HostApiError>;

    /// 根据窗口标题的部分内容激活已存在的窗口。
    /// 参数：title - 窗口标题的部分匹配文本。
    /// 返回：成功激活返回 Ok(true)，未找到窗口返回 Ok(false)，失败返回 HostApiError。
    async fn activate_window_by_title(&self, title: &str) -> Result<bool, HostApiError>;

    // ===== 配置管理 =====

    /// 更新插件的 SDK 配置。
    /// 参数：config - 新的插件 SDK 配置。
    /// 返回：无。
    /// 特性：立即生效，影响后续所有服务调用。
    fn update_config(&self, config: PluginSdkConfig);

    // ===== 能力查询 =====

    /// 查询当前平台支持的能力集合。
    /// 参数：无。
    /// 返回：平台能力的不可变引用。
    fn capabilities(&self) -> &PlatformCapabilities;
}

/// 宿主向插件暴露的平台能力注册层。
/// 插件必须先调用 register() 获取 PluginHandle，再通过句柄访问服务。
/// 全局管理操作（如更新缓存目录）保留在 HostApi 上，不暴露给插件。
#[async_trait]
pub trait HostApi: Send + Sync {
    /// 注册插件并返回绑定了插件身份与配置的服务句柄。
    /// 参数：plugin_id - 插件唯一标识；config - 插件的 SDK 配置。
    /// 返回：绑定该插件配置的服务句柄。
    /// 特性：同一 plugin_id 重复注册将覆盖原有配置。
    fn register(&self, plugin_id: &str, config: PluginSdkConfig) -> Arc<dyn PluginHandle>;

    /// 更新图标缓存目录路径（宿主级操作，不暴露给插件）。
    /// 参数：new_icon_cache_dir - 新的图标文件缓存目录路径。
    /// 返回：成功返回 Ok(())，失败返回 HostApiError。
    /// 特性：切换文件缓存的存储位置，同时清空内存缓存以保持一致性。
    async fn update_icon_cache_dir(&self, new_icon_cache_dir: &str) -> Result<(), HostApiError>;

    /// 查询当前平台支持的能力集合。
    /// 参数：无。
    /// 返回：平台能力的不可变引用。
    fn capabilities(&self) -> &PlatformCapabilities;
}
