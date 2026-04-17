use crate::sdk::host_api::HostApiError;
use async_trait::async_trait;

/// 应用启动器 trait，定义平台原语。
/// 各平台实现通过平台专属 API 启动应用，插件通过 PluginHandle 委托调用。
#[async_trait]
pub trait AppLauncher: Send + Sync {
    /// 启动应用。
    /// 参数：app_id - 应用唯一标识；args - 启动参数（可选）。
    /// 返回：成功返回 Ok(pid)，失败返回 HostApiError。
    async fn launch_app(&self, app_id: &str, args: Option<&[String]>) -> Result<u32, HostApiError>;
}
