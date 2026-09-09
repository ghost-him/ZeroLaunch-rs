use std::process::Command;
use zerolaunch_plugin_api::services::parameter::provider::{
    ProviderError, SystemParameterProvider,
};
/// Provides the current macOS clipboard text to parameterized actions.
pub struct MacosClipboardProvider;
/// Provides the Unix process identifier of the frontmost macOS application.
pub struct MacosWindowHandleProvider;
/// Provides selected text when macOS exposes it to the host application.
pub struct MacosSelectionProvider;
#[async_trait::async_trait]
impl SystemParameterProvider for MacosClipboardProvider {
    async fn get_value(&self) -> Result<String, ProviderError> {
        Ok(arboard::Clipboard::new()
            .and_then(|mut c| c.get_text())
            .unwrap_or_default())
    }
}
#[async_trait::async_trait]
impl SystemParameterProvider for MacosWindowHandleProvider {
    async fn get_value(&self) -> Result<String, ProviderError> {
        Ok(Command::new("osascript").args(["-e", "tell application \"System Events\" to unix id of first application process whose frontmost is true"]).output().ok().filter(|o| o.status.success()).map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned()).unwrap_or_else(|| "0".into()))
    }
}
#[async_trait::async_trait]
impl SystemParameterProvider for MacosSelectionProvider {
    async fn get_value(&self) -> Result<String, ProviderError> {
        Ok(String::new())
    }
}
