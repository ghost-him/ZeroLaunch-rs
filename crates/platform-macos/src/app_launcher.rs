use async_trait::async_trait;
use std::process::Command;
use zerolaunch_plugin_api::host::HostApiError;
use zerolaunch_plugin_api::services::app::AppLauncher;

/// Launches macOS applications through the `open` command.
///
/// Used by application executors through the platform abstraction.
pub struct MacosAppLauncher;
impl MacosAppLauncher {
    pub fn new() -> Self {
        Self
    }
}
impl Default for MacosAppLauncher {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AppLauncher for MacosAppLauncher {
    async fn launch_app(&self, app_id: &str, args: Option<&[String]>) -> Result<u32, HostApiError> {
        let mut command = Command::new("open");
        if app_id.contains('/') {
            command.arg(app_id);
        } else {
            command.args(["-b", app_id]);
        }
        if let Some(args) = args.filter(|args| !args.is_empty()) {
            command.arg("--args").args(args);
        }
        command
            .spawn()
            .map(|child| child.id())
            .map_err(|error| HostApiError::AppLaunchFailed {
                app_id: app_id.into(),
                reason: error.to_string(),
            })
    }
}
