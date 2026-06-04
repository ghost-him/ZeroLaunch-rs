use std::path::Path;
use std::process::Stdio;
use tokio::io::{BufReader, BufWriter};
use tokio::process::Child;
use zerolaunch_plugin_protocol::ProtocolError;

pub struct StdioTransport {
    pub child: Child,
    pub stdin: BufWriter<tokio::process::ChildStdin>,
    pub stdout: BufReader<tokio::process::ChildStdout>,
    pub stderr: tokio::process::ChildStderr,
}

impl StdioTransport {
    pub async fn spawn(
        command: &Path,
        args: &[String],
        cwd: &Path,
        env: &[(String, String)],
    ) -> Result<Self, ProtocolError> {
        let mut cmd = tokio::process::Command::new(command);
        cmd.args(args)
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        for (k, v) in env {
            cmd.env(k, v);
        }

        let mut child = cmd.spawn().map_err(|e| {
            ProtocolError::InvalidFrame(format!("failed to spawn '{}': {}", command.display(), e))
        })?;

        let stdin = BufWriter::new(
            child
                .stdin
                .take()
                .ok_or_else(|| ProtocolError::InvalidFrame("stdin not available".into()))?,
        );
        let stdout = BufReader::new(
            child
                .stdout
                .take()
                .ok_or_else(|| ProtocolError::InvalidFrame("stdout not available".into()))?,
        );
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| ProtocolError::InvalidFrame("stderr not available".into()))?;

        Ok(Self {
            child,
            stdin,
            stdout,
            stderr,
        })
    }

    pub fn pid(&self) -> Option<u32> {
        self.child.id()
    }
}
