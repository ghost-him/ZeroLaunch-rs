use crate::host::HostApiError;

/// 剪贴板管理器 trait，定义平台剪贴板写入原语。
/// 各平台实现通过系统 API 写入系统剪贴板。
pub trait ClipboardManager: Send + Sync {
    /// 将文本写入系统剪贴板。
    /// 参数：text - 要写入的文本内容。
    /// 返回：成功返回 Ok(())，失败返回 HostApiError。
    fn set_text(&self, text: &str) -> Result<(), HostApiError>;
}
