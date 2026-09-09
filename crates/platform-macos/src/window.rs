use async_trait::async_trait;
use std::process::Command;
use zerolaunch_plugin_api::host::HostApiError;
use zerolaunch_plugin_api::services::window::WindowManager;
/// Activates macOS application windows through AppleScript.
///
/// Used by window activation executors through the platform abstraction.
pub struct MacosWindowManager;
impl MacosWindowManager {
    pub fn new() -> Self {
        Self
    }
}
impl Default for MacosWindowManager {
    fn default() -> Self {
        Self::new()
    }
}
fn osascript(script: String) -> Result<bool, HostApiError> {
    Command::new("osascript")
        .args(["-e", &script])
        .status()
        .map(|s| s.success())
        .map_err(|e| HostApiError::WindowOperationFailed {
            detail: e.to_string(),
        })
}
fn apple_quote(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
#[async_trait]
impl WindowManager for MacosWindowManager {
    async fn activate_window_by_process(&self, process_name: &str) -> Result<bool, HostApiError> {
        osascript(format!(
            "tell application \"System Events\" to set frontmost of first application process whose name is \"{}\" to true",
            apple_quote(process_name)
        ))
    }
    async fn activate_window_by_title(&self, title: &str) -> Result<bool, HostApiError> {
        osascript(format!("tell application \"System Events\" to set frontmost of first application process whose name contains \"{}\" to true", apple_quote(title)))
    }
    async fn activate_window_by_pid(&self, pid: u32) -> Result<bool, HostApiError> {
        osascript(format!("tell application \"System Events\" to set frontmost of first application process whose unix id is {} to true", pid))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quotes_applescript_literals() {
        assert_eq!(apple_quote("A \"quoted\" app"), "A \\\"quoted\\\" app");
    }
}
