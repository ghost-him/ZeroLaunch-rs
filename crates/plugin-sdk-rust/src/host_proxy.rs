//! HostProxy — provides methods for third-party plugins to call host/* APIs.
//!
//! Each method sends an LSP-framed JSON-RPC request via the shared outbound
//! channel and awaits the response via a oneshot registered in the shared
//! pending map. This design avoids the deadlock of the old synchronous
//! stdin-lock approach by centralizing stdin reads and stdout writes into
//! dedicated async tasks in `runtime.rs`.

use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};

/// Proxy for calling host-side APIs from a plugin subprocess.
/// Does NOT access stdin/stdout directly — uses channel-based I/O.
pub struct HostProxy {
    next_id: AtomicU64,
    pending: Arc<DashMap<u64, oneshot::Sender<serde_json::Value>>>,
    outbound_tx: mpsc::Sender<Vec<u8>>,
}

impl HostProxy {
    pub fn new(
        pending: Arc<DashMap<u64, oneshot::Sender<serde_json::Value>>>,
        outbound_tx: mpsc::Sender<Vec<u8>>,
    ) -> Self {
        Self {
            next_id: AtomicU64::new(1),
            pending,
            outbound_tx,
        }
    }

    /// Send a host/* request via the shared stdout channel and await the response
    /// through the shared pending map.
    async fn send_request(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        let payload = serde_json::to_vec(&request).map_err(|e| e.to_string())?;
        let header = format!("Content-Length: {}\r\n\r\n", payload.len());

        // Combine header + payload into one write to avoid interleaving
        let mut frame = header.into_bytes();
        frame.extend_from_slice(&payload);

        // Register pending response
        let (tx, rx) = oneshot::channel();
        self.pending.insert(id, tx);

        // Send via the shared channel (write_task writes to stdout)
        self.outbound_tx
            .send(frame)
            .await
            .map_err(|_| "write channel closed".to_string())?;

        // Await the response (read_task completes the oneshot with resp.result).
        // Apply a 30-second timeout so the plugin doesn't hang forever if
        // the host crashes during request processing.
        tokio::time::timeout(Duration::from_secs(30), rx)
            .await
            .map_err(|_| "host call timed out".to_string())?
            .map_err(|_| "response channel closed".to_string())
    }

    pub async fn log(&self, level: &str, message: &str) -> Result<(), String> {
        self.send_request(
            "host/log",
            serde_json::json!({ "level": level, "message": message }),
        )
        .await?;
        Ok(())
    }

    pub async fn shell_open(&self, target: &str) -> Result<(), String> {
        self.send_request("host/shell.open", serde_json::json!({ "target": target }))
            .await?;
        Ok(())
    }

    pub async fn get_icon(&self, path: &str) -> Result<String, String> {
        let result = self
            .send_request(
                "host/icon.get",
                serde_json::json!({ "request": { "path": path }, "level": "Full" }),
            )
            .await?;
        Ok(result.as_str().unwrap_or("").to_string())
    }

    pub async fn shell_execute_command(&self, cmd: &str) -> Result<(), String> {
        self.send_request(
            "host/shell.execute_command",
            serde_json::json!({ "cmd": cmd }),
        )
        .await?;
        Ok(())
    }

    pub async fn shell_open_folder(&self, path: &str) -> Result<(), String> {
        self.send_request(
            "host/shell.open_folder",
            serde_json::json!({ "path": path }),
        )
        .await?;
        Ok(())
    }

    pub async fn shell_execute_elevation(&self, path: &str) -> Result<(), String> {
        self.send_request(
            "host/shell.execute_elevation",
            serde_json::json!({ "path": path }),
        )
        .await?;
        Ok(())
    }

    pub async fn notify(&self, title: &str, message: &str) -> Result<(), String> {
        self.send_request(
            "host/notify",
            serde_json::json!({ "title": title, "message": message }),
        )
        .await?;
        Ok(())
    }

    pub async fn enumerate_apps(&self) -> Result<serde_json::Value, String> {
        self.send_request("host/app.enumerate", serde_json::json!(null))
            .await
    }

    pub async fn resolve_path(&self, kind: &str) -> Result<String, String> {
        let result = self
            .send_request("host/path.resolve", serde_json::json!({ "kind": kind }))
            .await?;
        Ok(result.as_str().unwrap_or("").to_string())
    }

    pub async fn resource_upload(
        &self,
        plugin_id: &str,
        key: &str,
        bytes_b64: &str,
    ) -> Result<String, String> {
        let result = self
            .send_request(
                "host/resource.upload",
                serde_json::json!({
                    "pluginId": plugin_id,
                    "key": key,
                    "bytesB64": bytes_b64,
                }),
            )
            .await?;
        Ok(result.as_str().unwrap_or("").to_string())
    }

    pub async fn resource_get(&self, plugin_id: &str, key: &str) -> Result<String, String> {
        let result = self
            .send_request(
                "host/resource.get",
                serde_json::json!({
                    "pluginId": plugin_id,
                    "key": key,
                }),
            )
            .await?;
        Ok(result.as_str().unwrap_or("").to_string())
    }
}
