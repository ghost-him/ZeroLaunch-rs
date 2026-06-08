//! JSON-RPC runtime for third-party Rust plugins.
//!
//! The runtime uses a three-task async architecture:
//! - `read_task`: 唯一 stdin 读取者，解析 LSP 帧，路由响应到 pending_map，
//!   转发请求到 dispatch_task。
//! - `write_task`: 唯一 stdout 写入者，将所有出站消息编码为 LSP 帧。
//! - `dispatch_task`: 处理 plugin/* 请求，调用用户 Plugin trait 实现，
//!   将响应发到 write_task。
//!
//! HostProxy 通过共享的 pending_map 和 outbound_tx 发送 host/* 请求，
//! 避免了同步 BufReader 造成的死锁问题。

use dashmap::DashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, oneshot};

use zerolaunch_plugin_api::Plugin;
use zerolaunch_plugin_protocol::jsonrpc::{Message, Request, Response};
use zerolaunch_plugin_protocol::messages::*;
use zerolaunch_plugin_protocol::methods::plugin as plugin_methods;
use zerolaunch_plugin_protocol::{codes, JsonRpcError, PROTOCOL_VERSION};

use crate::host_proxy::HostProxy;

// Tokio task-local HostProxy, initialized by `run()`.
// All tasks spawned within the scope of `run_async` inherit this value.
tokio::task_local! {
    static HOST_PROXY: Arc<HostProxy>;
}

/// Returns the task-local `HostProxy` for the current `run()` scope.
/// Panics if called outside of `run()`.
pub fn host() -> Arc<HostProxy> {
    HOST_PROXY.with(|h| h.clone())
}

/// An incoming JSON-RPC request routed from the read task to the dispatch task.
struct IncomingRequest {
    id: u64,
    method: String,
    params: serde_json::Value,
}

/// Run the plugin JSON-RPC stdio loop with the given Plugin impl.
pub fn run(plugin: impl Plugin + 'static) {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("failed to build tokio runtime");

    rt.block_on(async move {
        run_async(plugin).await;
    });
}

async fn run_async(mut plugin: impl Plugin + 'static) {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    // Channels
    let (request_tx, mut request_rx) = mpsc::channel::<IncomingRequest>(64);
    let (outbound_tx, mut outbound_rx) = mpsc::channel::<Vec<u8>>(64);
    let pending: Arc<DashMap<u64, oneshot::Sender<serde_json::Value>>> = Arc::new(DashMap::new());

    // Create the HostProxy. When the scope exits, HOST_PROXY is dropped,
    // which drops outbound_tx's last clone, allowing the write task to
    // exit gracefully through channel closure.
    let host_proxy = Arc::new(HostProxy::new(pending.clone(), outbound_tx.clone()));

    HOST_PROXY
        .scope(host_proxy, async move {
            // Plugin state
            let mut plugin_context: Option<zerolaunch_plugin_api::PluginContext> = None;

            // --- Read task: stdin → pending_map (responses) or request_tx (new requests) ---
            let pending_r = pending.clone();
            let request_tx_clone = request_tx.clone();
            let read_handle = tokio::spawn(async move {
                let reader = BufReader::new(stdin);
                let mut stdin = reader;
                loop {
                    match read_frame(&mut stdin).await {
                        Ok(body) => {
                            let msg: Message = match serde_json::from_slice(&body) {
                                Ok(m) => m,
                                Err(_) => continue,
                            };
                            match msg {
                                Message::Response(resp) => {
                                    if let Some((_, tx)) = pending_r.remove(&resp.id) {
                                        let result = resp
                                            .result
                                            .or(resp
                                                .error
                                                .map(|e| serde_json::Value::String(e.message)))
                                            .unwrap_or(serde_json::Value::Null);
                                        let _ = tx.send(result);
                                    }
                                }
                                Message::Request(req) => {
                                    let _ = request_tx_clone
                                        .send(IncomingRequest {
                                            id: req.id,
                                            method: req.method,
                                            params: req.params,
                                        })
                                        .await;
                                }
                                Message::Notification(_) => {
                                    tracing::trace!("ignored notification");
                                }
                            }
                        }
                        Err(_) => break,
                    }
                }
            });

            // --- Dispatch task: plugin/* requests → user Plugin → responses to outbound_tx ---
            let outbound_dispatch = outbound_tx.clone();
            let dispatch_handle = tokio::spawn(async move {
                while let Some(incoming) = request_rx.recv().await {
                    let req = Request::new(incoming.id, &incoming.method, incoming.params);
                    let result = handle_request(&mut plugin, &req, &mut plugin_context).await;
                    if let Ok(payload) = serde_json::to_vec(&result) {
                        if outbound_dispatch.send(payload).await.is_err() {
                            break;
                        }
                    }
                }
            });

            // --- Write task: outbound_rx → stdout ---
            let write_handle = tokio::spawn(async move {
                let mut writer = stdout;
                while let Some(payload) = outbound_rx.recv().await {
                    let header = format!("Content-Length: {}\r\n\r\n", payload.len());
                    if writer.write_all(header.as_bytes()).await.is_err() {
                        break;
                    }
                    if writer.write_all(&payload).await.is_err() {
                        break;
                    }
                    if writer.flush().await.is_err() {
                        break;
                    }
                }
            });

            // Wait for read task to finish (transport closes).
            let _ = read_handle.await;

            // Drop request_tx → dispatch_task exits gracefully via
            // channel closure when it finishes the current request.
            drop(request_tx);

            // Write task cannot exit via channel closure because
            // the HOST_PROXY (inside this scope) still holds an
            // outbound_tx clone. We abort it.
            write_handle.abort();

            let _ = tokio::join!(dispatch_handle, write_handle);
        })
        .await;
    // HOST_PROXY scope ends here → Arc<HostProxy> dropped → final cleanup.
}

/// Read a single LSP-style Content-Length framed message.
/// Returns the raw JSON bytes, or an error string on parse/size/io failure.
async fn read_frame<R: tokio::io::AsyncBufRead + Unpin>(reader: &mut R) -> Result<Vec<u8>, String> {
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        if reader
            .read_line(&mut line)
            .await
            .map_err(|e| format!("read error: {}", e))?
            == 0
        {
            return Err("transport closed".into());
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            break;
        }
        if let Some(value) = trimmed.strip_prefix("Content-Length:") {
            content_length = value.trim().parse().ok();
        }
    }
    let len = content_length.ok_or("missing Content-Length")?;
    if len > 16 * 1024 * 1024 {
        return Err(format!("Content-Length too large: {}", len));
    }
    let mut body = vec![0u8; len];
    reader
        .read_exact(&mut body)
        .await
        .map_err(|e| format!("read body: {}", e))?;
    Ok(body)
}

async fn handle_request(
    plugin: &mut impl Plugin,
    req: &Request,
    plugin_ctx: &mut Option<zerolaunch_plugin_api::PluginContext>,
) -> Message {
    let id = req.id;
    let result = dispatch(plugin, &req.method, &req.params, plugin_ctx).await;
    match result {
        Ok(value) => Message::Response(Response::ok(id, value)),
        Err(err) => Message::Response(Response::err(id, err)),
    }
}

async fn dispatch(
    plugin: &mut impl Plugin,
    method: &str,
    params: &serde_json::Value,
    plugin_ctx: &mut Option<zerolaunch_plugin_api::PluginContext>,
) -> Result<serde_json::Value, JsonRpcError> {
    match method {
        plugin_methods::INITIALIZE => {
            let p: InitializeParams = serde_json::from_value(params.clone())
                .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
            *plugin_ctx = Some(zerolaunch_plugin_api::PluginContext {
                trace_id: "init".into(),
                query_id: None,
                plugin_id: Some(p.plugin_id),
            });
            let result = InitializeResult {
                plugin_version: plugin.metadata().version.clone(),
                protocol_version: PROTOCOL_VERSION.to_string(),
            };
            Ok(serde_json::to_value(result).unwrap_or_default())
        }
        plugin_methods::SHUTDOWN => Ok(serde_json::Value::Null),
        plugin_methods::GET_METADATA => {
            Ok(serde_json::to_value(plugin.metadata()).unwrap_or(serde_json::Value::Null))
        }
        plugin_methods::GET_COMPONENTS => {
            let components = vec![ComponentDescriptor {
                component_id: plugin.component_id().to_string(),
                component_name: plugin.component_name().to_string(),
                component_type: plugin.component_type(),
                kind: ComponentKind::Plugin {
                    trigger_keywords: plugin.metadata().trigger_keywords.clone(),
                },
                priority: plugin.metadata().priority,
            }];
            Ok(serde_json::to_value(components).unwrap_or_default())
        }
        plugin_methods::GET_SETTINGS_SCHEMA => {
            Ok(serde_json::to_value(plugin.setting_schema()).unwrap_or(serde_json::Value::Null))
        }
        plugin_methods::GET_SETTINGS => Ok(plugin.get_settings()),
        plugin_methods::APPLY_SETTINGS => {
            let p: ApplySettingsParams = serde_json::from_value(params.clone())
                .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
            plugin
                .apply_settings(p.settings)
                .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
            Ok(serde_json::Value::Null)
        }
        plugin_methods::VALIDATE_SETTINGS => {
            let p: ValidateSettingsParams = serde_json::from_value(params.clone())
                .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
            let result = match plugin.validate_settings(&p.settings) {
                Ok(()) => ValidateSettingsResult { error: None },
                Err(e) => ValidateSettingsResult {
                    error: Some(e.to_string()),
                },
            };
            Ok(serde_json::to_value(result).unwrap_or_default())
        }
        plugin_methods::CONFIG_ACTIONS => Ok(serde_json::to_value(ConfigActionsResult {
            actions: plugin.config_actions(),
        })
        .unwrap_or_default()),
        plugin_methods::EXECUTE_CONFIG_ACTION => {
            let p: ExecuteConfigActionParams = serde_json::from_value(params.clone())
                .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
            plugin
                .execute_config_action(&p.action, &p.params)
                .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))
        }
        plugin_methods::QUERY => {
            let p: QueryParams = serde_json::from_value(params.clone())
                .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
            let response = plugin
                .query(&p.ctx, &p.query)
                .await
                .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
            Ok(serde_json::to_value(response).unwrap_or_default())
        }
        plugin_methods::EXECUTE_ACTION => {
            let p: ExecuteActionParams = serde_json::from_value(params.clone())
                .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
            plugin
                .execute_action(&p.ctx, &p.action_id, p.payload)
                .await
                .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
            Ok(serde_json::Value::Null)
        }
        _ => Err(JsonRpcError::new(
            codes::METHOD_NOT_FOUND,
            format!("method not found: {}", method),
        )),
    }
}
