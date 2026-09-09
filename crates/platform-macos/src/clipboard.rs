use zerolaunch_plugin_api::host::HostApiError;
use zerolaunch_plugin_api::services::clipboard::ClipboardManager;
/// Writes text to the macOS user clipboard.
///
/// Used by executor and parameter-provider services through the platform abstraction.
pub struct MacosClipboardManager;
impl MacosClipboardManager {
    pub fn new() -> Self {
        Self
    }
}
impl Default for MacosClipboardManager {
    fn default() -> Self {
        Self::new()
    }
}
impl ClipboardManager for MacosClipboardManager {
    fn set_text(&self, text: &str) -> Result<(), HostApiError> {
        let mut clipboard =
            arboard::Clipboard::new().map_err(|e| HostApiError::ExecutionFailed {
                service: "clipboard".into(),
                reason: e.to_string(),
            })?;
        clipboard
            .set_text(text)
            .map_err(|e| HostApiError::ExecutionFailed {
                service: "clipboard".into(),
                reason: e.to_string(),
            })
    }
}
