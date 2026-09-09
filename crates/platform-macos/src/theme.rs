use std::process::Command;
use std::sync::{Arc, OnceLock};
use std::thread;
use std::time::Duration;
use zerolaunch_plugin_api::host::HostApiError;
use zerolaunch_plugin_api::services::{Theme, ThemeProvider};
/// Reads and monitors the macOS light or dark appearance setting.
///
/// Used by the frontend theme bridge and appearance configuration.
pub struct MacosThemeProvider;
impl MacosThemeProvider {
    pub fn new() -> Self {
        Self
    }
}
impl Default for MacosThemeProvider {
    fn default() -> Self {
        Self::new()
    }
}
pub(crate) fn parse_theme(value: &str) -> Theme {
    if value.trim().eq_ignore_ascii_case("dark") {
        Theme::Dark
    } else {
        Theme::Light
    }
}
impl ThemeProvider for MacosThemeProvider {
    fn current_system_theme(&self) -> Result<Theme, HostApiError> {
        let output = Command::new("defaults")
            .args(["read", "-g", "AppleInterfaceStyle"])
            .output()
            .map_err(|e| HostApiError::ExecutionFailed {
                service: "theme".into(),
                reason: e.to_string(),
            })?;
        Ok(parse_theme(&String::from_utf8_lossy(&output.stdout)))
    }
}
pub fn start_system_theme_monitor(
    callback: impl Fn(Theme) + Send + Sync + 'static,
) -> Result<(), String> {
    static STARTED: OnceLock<()> = OnceLock::new();
    if STARTED.set(()).is_err() {
        return Ok(());
    }
    let callback: Arc<dyn Fn(Theme) + Send + Sync> = Arc::new(callback);
    let handle = thread::Builder::new()
        .name("macos-theme-monitor".into())
        .spawn(move || {
            let provider = MacosThemeProvider;
            let mut previous = provider.current_system_theme().unwrap_or(Theme::Light);
            loop {
                thread::sleep(Duration::from_secs(2));
                let current = provider.current_system_theme().unwrap_or(previous);
                if current != previous {
                    previous = current;
                    callback(current);
                }
            }
        });
    if let Err(error) = handle {
        return Err(format!("failed to spawn theme monitor thread: {error}"));
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_defaults_dark_value() {
        assert_eq!(parse_theme("Dark\n"), Theme::Dark);
        assert_eq!(parse_theme(""), Theme::Light);
    }
}
