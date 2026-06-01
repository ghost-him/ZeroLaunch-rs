use crate::host::error::HostApiError;
use async_trait::async_trait;

/// 自启动管理器 trait，定义平台无关的自启动能力契约。
/// 各平台实现此 trait，处理平台特定的自启动逻辑。
#[async_trait]
pub trait AutoStartManager: Send + Sync {
    /// 启用自启动
    async fn enable(&self, task_name: &str, exe_path: &str) -> Result<(), HostApiError>;
    /// 禁用自启动
    async fn disable(&self, task_name: &str) -> Result<(), HostApiError>;
    /// 检查自启动是否已启用
    async fn is_enabled(&self, task_name: &str) -> Result<bool, HostApiError>;
    /// 获取默认任务名称（平台特定）
    fn default_task_name(&self) -> String;
}
