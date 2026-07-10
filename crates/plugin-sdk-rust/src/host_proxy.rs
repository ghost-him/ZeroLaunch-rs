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

use zerolaunch_plugin_protocol::codec::encode_frame;

use base64::Engine as _;

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

        // 编码为完整的 LSP Content-Length 帧，避免 header 和 body 交错
        let frame = encode_frame(&payload);

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

    /// 发送 host/log 请求但不等待响应（fire-and-forget）。
    ///
    /// pending 条目在宿主响应到达时由 read_task 自动清理。
    /// 若 outbound 通道已满，日志被静默丢弃并从 pending 中移除，
    /// 避免阻塞调用者（通常来自 tracing subscriber 的回调）。
    pub fn log_no_wait(&self, level: &str, message: &str) {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let Ok(payload) = serde_json::to_vec(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "host/log",
            "params": { "level": level, "message": message },
        })) else {
            return;
        };

        let frame = encode_frame(&payload);
        let (tx, _rx) = oneshot::channel(); // _rx 立即 drop → fire-and-forget
        self.pending.insert(id, tx);

        // 非阻塞投递：通道满了则丢弃并清理 pending
        if self.outbound_tx.try_send(frame).is_err() {
            self.pending.remove(&id);
        }
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

    /// 上传插件本地文件到宿主资源空间。
    /// 直接传递文件路径，由宿主负责读取。
    pub async fn resource_upload(
        &self,
        resource_id: &str,
        file_path: &str,
        max_size: Option<u64>,
    ) -> Result<String, String> {
        let result = self
            .send_request(
                "host/resource.upload",
                serde_json::json!({
                    "resourceId": resource_id,
                    "filePath": file_path,
                    "maxSize": max_size,
                }),
            )
            .await?;
        Ok(result.as_str().unwrap_or("").to_string())
    }

    pub async fn resource_get(&self, resource_id: &str) -> Result<Vec<u8>, String> {
        let result = self
            .send_request(
                "host/resource.get",
                serde_json::json!({
                    "resourceId": resource_id,
                }),
            )
            .await?;
        let b64 = result.as_str().unwrap_or("");
        base64::engine::general_purpose::STANDARD
            .decode(b64)
            .map_err(|e| format!("base64 decode failed: {}", e))
    }

    /// 直接写入资源字节数据（无需临时文件），base64 编解码由 SDK 内部处理。
    pub async fn resource_put(&self, resource_id: &str, data: &[u8]) -> Result<(), String> {
        let b64 = base64::engine::general_purpose::STANDARD.encode(data);
        self.send_request(
            "host/resource.put",
            serde_json::json!({
                "resourceId": resource_id,
                "bytesB64": b64,
            }),
        )
        .await?;
        Ok(())
    }

    /// 删除资源文件。
    pub async fn resource_delete(&self, resource_id: &str) -> Result<(), String> {
        self.send_request(
            "host/resource.delete",
            serde_json::json!({
                "resourceId": resource_id,
            }),
        )
        .await?;
        Ok(())
    }

    /// 列出本插件的所有资源标识符。
    pub async fn resource_list(&self) -> Result<Vec<String>, String> {
        let result = self
            .send_request("host/resource.list", serde_json::json!({}))
            .await?;
        serde_json::from_value(result).map_err(|e| format!("parse resource list failed: {}", e))
    }
}
