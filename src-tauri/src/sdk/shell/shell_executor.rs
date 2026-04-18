use crate::sdk::host_api::{HostApiError, OpenTarget};
use async_trait::async_trait;

/// Shell 执行器 trait，定义平台原语。
/// 平台实现者实现各原语方法，PluginHandle 通过注入的 ShellExecutor 委托调用。
#[async_trait]
pub trait ShellExecutor: Send + Sync {
    /// 使用系统默认方式打开目标（文件/网址/文件夹）。
    /// 参数：target - 打开目标枚举。
    /// 返回：成功返回 Ok(())，失败返回 HostApiError。
    async fn shell_open(&self, target: &OpenTarget) -> Result<(), HostApiError>;

    /// 在文件资源管理器中打开指定路径的父目录并选中该文件。
    /// 参数：path - 要打开所在位置的文件路径。
    /// 返回：成功返回 Ok(())，失败返回 HostApiError。
    async fn shell_open_folder(&self, path: &str) -> Result<(), HostApiError>;

    /// 以管理员权限启动程序。
    /// 参数：path - 要以管理员身份运行的程序路径。
    /// 返回：成功返回 Ok(())，失败返回 HostApiError。
    async fn shell_execute_elevation(&self, path: &str) -> Result<(), HostApiError>;

    /// 执行命令字符串（后台运行，无窗口）。
    /// 参数：command - 要执行的命令字符串。
    /// 返回：成功返回 Ok(())，失败返回 HostApiError。
    async fn shell_execute_command(&self, command: &str) -> Result<(), HostApiError>;
}
