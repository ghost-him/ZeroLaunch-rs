//! 系统参数提供者 trait 定义

/// 系统参数提供者错误
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    /// 无法获取参数值
    #[error("无法获取参数值: {0}")]
    GetValueFailed(String),
}

/// 系统参数提供者 trait
///
/// 设计原则：
/// - 每个 Provider 负责获取一种系统参数
/// - 通过依赖注入到 HostApi，便于测试和跨平台实现
/// - 异步接口，支持需要 I/O 操作的参数获取
///
/// 架构说明：
/// - trait 定义在功能模块内，与 IconExtractor、ShellExecutor 等保持一致
/// - 平台实现在 sdk/platform/windows/parameter_providers.rs
#[async_trait::async_trait]
pub trait SystemParameterProvider: Send + Sync {
    /// 获取参数值
    ///
    /// 返回：参数值的字符串表示，失败返回 ProviderError
    async fn get_value(&self) -> Result<String, ProviderError>;
}
