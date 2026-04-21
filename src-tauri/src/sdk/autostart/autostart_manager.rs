use crate::sdk::host_api::HostApiError;
use async_trait::async_trait;

/// 自启动管理器 trait，定义平台无关的自启动能力契约。
/// 各平台实现此 trait，处理平台特定的自启动逻辑。
#[async_trait]
pub trait AutoStartManager: Send + Sync {
    /// 启用自启动
    ///
    /// 参数：
    /// - task_name: 任务名称（用于标识自启动任务）
    /// - exe_path: 要自启动的可执行文件路径
    ///
    /// 返回：成功返回 Ok(())，失败返回 HostApiError
    async fn enable(&self, task_name: &str, exe_path: &str) -> Result<(), HostApiError>;

    /// 禁用自启动
    ///
    /// 参数：
    /// - task_name: 任务名称
    ///
    /// 返回：成功返回 Ok(())，失败返回 HostApiError
    async fn disable(&self, task_name: &str) -> Result<(), HostApiError>;

    /// 检查自启动是否已启用
    ///
    /// 参数：
    /// - task_name: 任务名称
    ///
    /// 返回：已启用返回 Ok(true)，否则返回 Ok(false)，失败返回 HostApiError
    async fn is_enabled(&self, task_name: &str) -> Result<bool, HostApiError>;

    /// 获取默认任务名称（平台特定）
    ///
    /// 返回：平台特定的默认任务名称（通常包含用户名）
    fn default_task_name(&self) -> String;
}
