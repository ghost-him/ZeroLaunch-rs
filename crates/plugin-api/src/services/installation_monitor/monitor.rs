use crate::host::error::HostApiError;
use crate::services::installation_monitor::types::InstallationCallback;
use async_trait::async_trait;

/// 安装监控器 trait，定义平台无关的文件系统监控能力契约。
/// 各平台实现此 trait，处理平台特定的文件监控逻辑。
/// 支持多个回调注册，当监控目录发生变化时依次调用所有回调。
#[async_trait]
pub trait InstallationMonitor: Send + Sync {
    /// 开始监控文件系统变化。
    /// 启动后，当监控目录发生变化时，将依次调用所有已注册的回调。
    /// 返回：成功返回 Ok(())，失败返回 HostApiError。
    async fn start_watching(&self) -> Result<(), HostApiError>;

    /// 停止监控文件系统变化。
    /// 返回：成功返回 Ok(())，失败返回 HostApiError。
    async fn stop_watching(&self) -> Result<(), HostApiError>;

    /// 检查是否正在监控。
    /// 返回：正在监控返回 true，否则返回 false。
    fn is_watching(&self) -> bool;

    /// 注册安装事件回调。
    /// 参数：id - 回调标识（用于注销）；callback - 回调函数。
    fn register_callback(&self, id: &str, callback: InstallationCallback);

    /// 注销安装事件回调。
    /// 参数：id - 回调标识。
    fn unregister_callback(&self, id: &str);

    /// 更新监控路径列表。
    /// 参数：paths - 要监控的目录路径列表。
    fn update_watch_paths(&self, paths: Vec<String>);
}
