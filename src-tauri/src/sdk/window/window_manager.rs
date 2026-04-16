use crate::sdk::host_api::HostApiError;
use async_trait::async_trait;

/// 窗口管理器 trait，定义平台原语。
/// 平台实现者实现各原语方法，PluginHandle 通过注入的 WindowManager 委托调用。
#[async_trait]
pub trait WindowManager: Send + Sync {
    /// 根据进程名激活已存在的窗口。
    /// 参数：process_name - 进程名（如 "chrome.exe"，含扩展名）。
    /// 返回：成功激活返回 Ok(true)，未找到窗口返回 Ok(false)，失败返回 HostApiError。
    async fn activate_window_by_process(&self, process_name: &str) -> Result<bool, HostApiError>;

    /// 根据窗口标题的部分内容激活已存在的窗口。
    /// 参数：title - 窗口标题的部分匹配文本（不区分大小写）。
    /// 返回：成功激活返回 Ok(true)，未找到窗口返回 Ok(false)，失败返回 HostApiError。
    async fn activate_window_by_title(&self, title: &str) -> Result<bool, HostApiError>;
}
