use zerolaunch_plugin_api::host::HostApiError;
use zerolaunch_plugin_api::services::clipboard::ClipboardManager;

/// Windows 剪贴板管理器实现。
/// 基于 arboard 写入系统剪贴板。
pub struct WindowsClipboardManager;

impl WindowsClipboardManager {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WindowsClipboardManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ClipboardManager for WindowsClipboardManager {
    fn set_text(&self, text: &str) -> Result<(), HostApiError> {
        let mut clipboard =
            arboard::Clipboard::new().map_err(|e| HostApiError::ExecutionFailed {
                service: "clipboard".to_string(),
                reason: format!("剪贴板初始化失败: {}", e),
            })?;
        clipboard
            .set_text(text)
            .map_err(|e| HostApiError::ExecutionFailed {
                service: "clipboard".to_string(),
                reason: format!("剪贴板写入失败: {}", e),
            })
    }
}
