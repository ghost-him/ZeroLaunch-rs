use async_trait::async_trait;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use zerolaunch_plugin_api::host::HostApiError;
use zerolaunch_plugin_api::services::autostart::AutoStartManager;

/// Manages the application LaunchAgent used for macOS login startup.
///
/// Used by the general configuration component through the platform abstraction.
pub struct MacosAutoStartManager;
impl MacosAutoStartManager {
    pub fn new() -> Self {
        Self
    }
    fn launch_agents_dir() -> Result<PathBuf, HostApiError> {
        dirs::home_dir()
            .map(|p| p.join("Library/LaunchAgents"))
            .ok_or_else(|| HostApiError::AutoStartFailed {
                reason: "cannot determine home directory".into(),
            })
    }
    fn plist_path(task_name: &str) -> Result<PathBuf, HostApiError> {
        Ok(Self::launch_agents_dir()?.join(format!("{task_name}.plist")))
    }
}
impl Default for MacosAutoStartManager {
    fn default() -> Self {
        Self::new()
    }
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
pub(crate) fn launch_agent_plist(label: &str, executable: &str) -> String {
    format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?><!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\"><plist version=\"1.0\"><dict><key>Label</key><string>{}</string><key>ProgramArguments</key><array><string>{}</string></array><key>RunAtLoad</key><true/></dict></plist>", xml_escape(label), xml_escape(executable))
}

#[async_trait]
impl AutoStartManager for MacosAutoStartManager {
    async fn enable(&self, task_name: &str, exe_path: &str) -> Result<(), HostApiError> {
        let dir = Self::launch_agents_dir()?;
        fs::create_dir_all(&dir).map_err(|e| HostApiError::AutoStartFailed {
            reason: e.to_string(),
        })?;
        let path = Self::plist_path(task_name)?;
        fs::write(&path, launch_agent_plist(task_name, exe_path)).map_err(|e| {
            HostApiError::AutoStartFailed {
                reason: e.to_string(),
            }
        })?;
        let service = format!("gui/{}", current_uid());
        let launchctl = |args: &[&str]| {
            Command::new("launchctl").args(args).output().map_err(|e| {
                HostApiError::AutoStartFailed {
                    reason: format!("failed to run launchctl: {e}"),
                }
            })
        };
        // Boot out first so re-enabling is idempotent (failure means not loaded yet).
        let _ = launchctl(&["bootout", &service, path.to_string_lossy().as_ref()]);
        let output = launchctl(&["bootstrap", &service, path.to_string_lossy().as_ref()])?;
        if !output.status.success() {
            // Roll back the plist so is_enabled stays consistent with launchctl state.
            let _ = fs::remove_file(&path);
            return Err(HostApiError::AutoStartFailed {
                reason: format!(
                    "launchctl bootstrap failed: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                ),
            });
        }
        Ok(())
    }
    async fn disable(&self, task_name: &str) -> Result<(), HostApiError> {
        let path = Self::plist_path(task_name)?;
        let service = format!("gui/{}", current_uid());
        let output = Command::new("launchctl")
            .args(["bootout", &service, path.to_string_lossy().as_ref()])
            .output()
            .map_err(|e| HostApiError::AutoStartFailed {
                reason: format!("failed to run launchctl: {e}"),
            })?;
        if !output.status.success() {
            // LaunchAgent registration is driven by plist presence at next
            // login; a bootout failure means the service was already gone
            // (state drifted), so removing the plist converges the switch.
            tracing::warn!(
                "launchctl bootout failed, removing plist anyway: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        if path.exists() {
            fs::remove_file(path).map_err(|e| HostApiError::AutoStartFailed {
                reason: e.to_string(),
            })?;
        }
        Ok(())
    }
    async fn is_enabled(&self, task_name: &str) -> Result<bool, HostApiError> {
        Ok(Self::plist_path(task_name)?.is_file())
    }
    fn default_task_name(&self) -> String {
        "com.zerolaunch-rs.app".into()
    }
}

fn current_uid() -> String {
    Command::new("id")
        .arg("-u")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .unwrap_or_else(|| "0".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn plist_escapes_program_path() {
        let plist = launch_agent_plist("com.test", "/tmp/A & B");
        assert!(plist.contains("/tmp/A &amp; B"));
        assert!(plist.contains("<key>RunAtLoad</key><true/>"));
    }
}
