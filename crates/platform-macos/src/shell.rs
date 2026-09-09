use async_trait::async_trait;
use std::path::Path;
use std::process::Command;
use zerolaunch_plugin_api::host::{HostApiError, OpenTarget};
use zerolaunch_plugin_api::services::shell::ShellExecutor;

/// Opens files, folders, URLs, and shell commands using macOS facilities.
///
/// Used by shell and executor components through the platform abstraction.
pub struct MacosShellExecutor;
impl MacosShellExecutor {
    pub fn new() -> Self {
        Self
    }
}
impl Default for MacosShellExecutor {
    fn default() -> Self {
        Self::new()
    }
}

fn open(args: &[&str]) -> Result<(), HostApiError> {
    Command::new("open")
        .args(args)
        .spawn()
        .map(|_| ())
        .map_err(|error| HostApiError::ShellOperationFailed {
            target: args.last().unwrap_or(&"").to_string(),
            reason: error.to_string(),
        })
}

#[async_trait]
impl ShellExecutor for MacosShellExecutor {
    async fn shell_open(&self, target: &OpenTarget) -> Result<(), HostApiError> {
        match target {
            OpenTarget::File(p) | OpenTarget::Url(p) | OpenTarget::Folder(p) => open(&[p]),
        }
    }
    async fn shell_open_folder(&self, path: &str) -> Result<(), HostApiError> {
        if !Path::new(path).exists() {
            return Err(HostApiError::ShellOperationFailed {
                target: path.into(),
                reason: "path does not exist".into(),
            });
        }
        open(&["-R", path])
    }
    async fn shell_execute_elevation(&self, _path: &str) -> Result<(), HostApiError> {
        Err(HostApiError::UnsupportedCapability(
            zerolaunch_plugin_api::PlatformCapability::RunAsAdmin,
        ))
    }
    async fn shell_execute_command(&self, command: &str) -> Result<(), HostApiError> {
        Command::new("/bin/sh")
            .args(["-c", command])
            .spawn()
            .map(|_| ())
            .map_err(|error| HostApiError::ExecutionFailed {
                service: "shell".into(),
                reason: error.to_string(),
            })
    }
}
