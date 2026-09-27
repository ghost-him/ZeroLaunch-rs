//! HostProxy — 第三方插件调用 host/* API 的代理。
//!
//! 每个方法经共享出站通道发送 LSP 帧 JSON-RPC 请求，并通过注册在共享
//! pending map 中的 oneshot 等待响应。该设计将 stdin 读取与 stdout 写入
//! 集中到独立任务，避免旧同步 stdin 锁方案的死锁。
use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use zerolaunch_plugin_api::services::model::{
    ModelChatRequest, ModelChatResponse, ModelEmbeddingRequest, ModelEmbeddingResponse, ModelInfo,
    ModelSimilarityRequest, ModelSimilarityResponse,
};
use zerolaunch_plugin_protocol::methods::host;
use zerolaunch_plugin_protocol::JsonRpcError;

use base64::Engine as _;

/// Proxy for calling host-side APIs from a plugin subprocess.
/// Does NOT access stdin/stdout directly — uses channel-based I/O.
pub struct HostProxy {
    /// 请求 id 分配器（单调递增，从 1 开始）。
    next_id: AtomicU64,
    /// 在途请求表：id → 响应通道。read_task 完成响应后移除条目。
    /// 值类型为 `Result`：宿主错误经 Err(JsonRpcError) 返回，调用方区分错误与结果。
    pending: Arc<DashMap<u64, oneshot::Sender<Result<serde_json::Value, JsonRpcError>>>>,
    /// 出站帧通道：write_task 独占消费并写 stdout。
    outbound_tx: mpsc::Sender<Vec<u8>>,
}

impl HostProxy {
    pub fn new(
        pending: Arc<DashMap<u64, oneshot::Sender<Result<serde_json::Value, JsonRpcError>>>>,
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
        // 模型类方法耗时可能远超普通 host/*（LLM 生成 / 大批量 embedding），
        // 单独放宽超时；其余方法维持 30s。
        let is_model_call = method.starts_with("host/model.");
        let timeout = if is_model_call {
            Duration::from_secs(300)
        } else {
            Duration::from_secs(30)
        };
        self.send_request_inner(method, params, timeout).await
    }

    /// 发送请求并等待响应；超时/通道关闭返回错误字符串。
    async fn send_request_inner(
        &self,
        method: &str,
        params: serde_json::Value,
        timeout: Duration,
    ) -> Result<serde_json::Value, String> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        let payload = serde_json::to_vec(&request).map_err(|e| e.to_string())?;

        // Register pending response
        let (tx, rx) = oneshot::channel();
        self.pending.insert(id, tx);

        // Send via the shared channel (write_task writes to stdout).
        // 注意：此处只投递干净 JSON，分帧统一由 write_task 完成，
        // 若在此 encode_frame 会导致双重分帧（帧内嵌套帧），宿主无法解析。
        if self.outbound_tx.send(payload).await.is_err() {
            self.pending.remove(&id);
            return Err("write channel closed".to_string());
        }

        // Await the response (read_task completes the oneshot with Result).
        // Apply a timeout so the plugin doesn't hang forever if
        // the host crashes during request processing.
        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(Ok(value))) => Ok(value),
            Ok(Ok(Err(err))) => Err(format!(
                "host call failed (code {}): {}",
                err.code, err.message
            )),
            Ok(Err(_)) => Err("response channel closed".to_string()),
            Err(_) => {
                // 超时：移除 pending 条目，防止长驻插件无界泄漏。
                self.pending.remove(&id);
                Err("host call timed out".to_string())
            }
        }
    }

    pub async fn log(&self, level: &str, message: &str) -> Result<(), String> {
        self.send_request(
            host::LOG,
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
            "method": host::LOG,
            "params": { "level": level, "message": message },
        })) else {
            return;
        };

        let (tx, _rx) = oneshot::channel(); // _rx 立即 drop → fire-and-forget
        self.pending.insert(id, tx);

        // 非阻塞投递：通道满了则丢弃并清理 pending。
        // 同 send_request：只投递干净 JSON，分帧由 write_task 统一完成。
        if self.outbound_tx.try_send(payload).is_err() {
            self.pending.remove(&id);
        }
    }

    pub async fn shell_open(&self, target: &str) -> Result<(), String> {
        self.send_request(host::SHELL_OPEN, serde_json::json!({ "target": target }))
            .await?;
        Ok(())
    }

    /// 获取图标字节（base64 字符串）。
    /// 返回：图标字节的 base64（WebP，回退可能为 PNG）；失败为空字符串。
    pub async fn get_icon(&self, path: &str) -> Result<String, String> {
        let result = self
            .send_request(
                host::ICON_GET,
                serde_json::json!({ "request": { "path": path }, "level": "Full" }),
            )
            .await?;
        Ok(result.as_str().unwrap_or("").to_string())
    }

    pub async fn shell_execute_command(&self, cmd: &str) -> Result<(), String> {
        self.send_request(
            host::SHELL_EXECUTE_COMMAND,
            serde_json::json!({ "cmd": cmd }),
        )
        .await?;
        Ok(())
    }

    pub async fn shell_open_folder(&self, path: &str) -> Result<(), String> {
        self.send_request(host::SHELL_OPEN_FOLDER, serde_json::json!({ "path": path }))
            .await?;
        Ok(())
    }

    pub async fn shell_execute_elevation(&self, path: &str) -> Result<(), String> {
        self.send_request(
            host::SHELL_EXECUTE_ELEVATION,
            serde_json::json!({ "path": path }),
        )
        .await?;
        Ok(())
    }

    pub async fn notify(&self, title: &str, message: &str) -> Result<(), String> {
        self.send_request(
            host::NOTIFY,
            serde_json::json!({ "title": title, "message": message }),
        )
        .await?;
        Ok(())
    }

    /// 获取宿主当前界面语言（如 "zh-Hans"），供插件生成本地化文本。
    pub async fn get_locale(&self) -> Result<String, String> {
        let result = self
            .send_request(host::GET_LOCALE, serde_json::json!(null))
            .await?;
        Ok(result.as_str().unwrap_or("").to_string())
    }

    /// 查询宿主当前实际生效主题，返回 `light` 或 `dark`。
    pub async fn get_theme(&self) -> Result<String, String> {
        let result = self
            .send_request(host::GET_THEME, serde_json::Value::Null)
            .await?;
        result
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| "host theme response is not a string".to_string())
    }

    /// 全网模型清单（聚合所有提供方）。
    pub async fn model_list(&self) -> Result<Vec<ModelInfo>, String> {
        let result = self
            .send_request(host::MODEL_LIST, serde_json::Value::Null)
            .await?;
        serde_json::from_value(result).map_err(|e| e.to_string())
    }

    /// 按 model_id 调用文本生成。
    pub async fn model_chat(&self, req: ModelChatRequest) -> Result<ModelChatResponse, String> {
        let params = serde_json::to_value(req).map_err(|e| e.to_string())?;
        let result = self.send_request(host::MODEL_CHAT, params).await?;
        serde_json::from_value(result).map_err(|e| e.to_string())
    }

    /// 按 model_id 调用文本向量化（task_type 必填，宿主对缺失/未知值返回错误）。
    pub async fn model_embedding(
        &self,
        req: ModelEmbeddingRequest,
    ) -> Result<ModelEmbeddingResponse, String> {
        let params = serde_json::to_value(req).map_err(|e| e.to_string())?;
        let result = self.send_request(host::MODEL_EMBEDDING, params).await?;
        serde_json::from_value(result).map_err(|e| e.to_string())
    }

    /// 按 model_id 计算查询向量与多个目标向量的相似度。
    pub async fn model_similarity(
        &self,
        req: ModelSimilarityRequest,
    ) -> Result<ModelSimilarityResponse, String> {
        let params = serde_json::to_value(req).map_err(|e| e.to_string())?;
        let result = self.send_request(host::MODEL_SIMILARITY, params).await?;
        serde_json::from_value(result).map_err(|e| e.to_string())
    }

    pub async fn enumerate_apps(&self) -> Result<serde_json::Value, String> {
        self.send_request(host::APP_ENUMERATE, serde_json::json!(null))
            .await
    }

    pub async fn resolve_path(&self, kind: &str) -> Result<String, String> {
        let result = self
            .send_request(host::PATH_RESOLVE, serde_json::json!({ "kind": kind }))
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
                host::RESOURCE_UPLOAD,
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
                host::RESOURCE_GET,
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
            host::RESOURCE_PUT,
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
            host::RESOURCE_DELETE,
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
            .send_request(host::RESOURCE_LIST, serde_json::json!({}))
            .await?;
        serde_json::from_value(result).map_err(|e| format!("parse resource list failed: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_proxy() -> (Arc<HostProxy>, mpsc::Receiver<Vec<u8>>) {
        let (tx, rx) = mpsc::channel::<Vec<u8>>(16);
        let pending: Arc<DashMap<u64, oneshot::Sender<Result<serde_json::Value, JsonRpcError>>>> =
            Arc::new(DashMap::new());
        (Arc::new(HostProxy::new(pending, tx)), rx)
    }

    /// 回归测试：host/* 请求必须以**干净 JSON** 投递到 outbound 通道，
    /// 分帧由 write_task 统一完成。
    ///
    /// 修复前 send_request/log_no_wait 在投递前先 encode_frame，
    /// 导致双重分帧（帧内嵌帧）：写出的字节为
    /// `Content-Length: M\r\n\r\nContent-Length: N\r\n\r\n{JSON}`，
    /// 宿主 read_frame 后 body 以 `Content-Length:` 开头，serde_json 解析失败
    /// （`expected value at line 1 column 1`），host/* 调用全部超时。
    #[tokio::test]
    async fn send_request_posts_clean_json_without_embedded_frame() {
        let (proxy, mut rx) = make_proxy();

        // log_no_wait 是同步 fire-and-forget：直接检查通道里的字节。
        proxy.log_no_wait("warn", "test message");
        let bytes = rx.recv().await.expect("log_no_wait 应投递一条消息");
        let text = String::from_utf8(bytes.clone()).expect("UTF-8");
        assert!(
            !text.contains("Content-Length"),
            "outbound 通道中的消息不得包含帧头（双重分帧）: {:?}",
            text
        );
        // 必须是合法 JSON-RPC 消息
        let v: serde_json::Value = serde_json::from_slice(&bytes).expect("干净 JSON 可解析");
        assert_eq!(v["method"], host::LOG);
        assert_eq!(v["params"]["message"], "test message");
    }

    /// send_request 投递的也必须是干净 JSON（await 路径）。
    #[tokio::test]
    async fn send_request_await_path_posts_clean_json() {
        let (proxy, mut rx) = make_proxy();
        // send_request 会 await 响应（oneshot 永不完成→挂起），
        // 因此只在通道上取一条消息验证负载，然后丢弃任务。
        let task = tokio::spawn(async move {
            let _ = proxy.model_list().await; // 挂起直到超时（30s），任务随测试结束
        });
        let bytes = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("send_request 应投递一条消息")
            .expect("通道未关闭");
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            !text.contains("Content-Length"),
            "outbound 通道中的消息不得包含帧头（双重分帧）: {:?}",
            text
        );
        let v: serde_json::Value = serde_json::from_slice(&bytes).expect("干净 JSON 可解析");
        assert_eq!(v["method"], host::MODEL_LIST);
        // 清理：中止挂起的任务
        task.abort();
    }
}
