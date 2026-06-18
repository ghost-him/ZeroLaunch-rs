use crate::platform::capabilities::PlatformCapability;

/// HostApi 统一错误类型。
/// 涵盖能力查询失败、注册失败、各服务操作失败等场景。
#[derive(Debug, thiserror::Error)]
pub enum HostApiError {
    /// 当前平台不支持请求的能力
    #[error("平台不支持该能力: {0:?}")]
    UnsupportedCapability(PlatformCapability),

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

    /// 资源路径包含路径遍历字符 (如 "..")
    #[error("路径遍历被拒绝: {path}")]
    PathTraversalRejected { path: String },
}
