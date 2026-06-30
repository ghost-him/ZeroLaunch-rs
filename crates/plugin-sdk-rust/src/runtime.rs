//! 第三方 Rust 插件的 JSON-RPC 运行时。
//!
//! 运行时采用三任务异步架构：
//! - `read_task`：唯一 stdin 读取者，解析 LSP 帧，路由响应到 pending_map，
//!   转发请求到 dispatch_task。
//! - `write_task`：唯一 stdout 写入者，将所有出站消息编码为 LSP 帧。
//! - `dispatch_task`：处理 plugin/* 请求，调用用户 Plugin trait 实现，
//!   将响应发到 write_task。
//!
//! HostProxy 通过共享的 pending_map 和 outbound_tx 发送 host/* 请求，
//! 避免了同步 BufReader 造成的死锁问题。

use dashmap::DashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, oneshot};

use zerolaunch_plugin_api::Plugin;
use zerolaunch_plugin_protocol::codec::{encode_frame, MAX_FRAME_SIZE, MAX_HEADER_SIZE};
use zerolaunch_plugin_protocol::jsonrpc::{Message, Request, Response};
use zerolaunch_plugin_protocol::messages::*;
use zerolaunch_plugin_protocol::methods::plugin as plugin_methods;
use zerolaunch_plugin_protocol::{codes, JsonRpcError, PROTOCOL_VERSION};

use crate::host_proxy::HostProxy;

// Tokio task-local HostProxy，由 `run()` 初始化。
// 在 `run_async` scope 内 spawn 的所有任务都继承该值。
tokio::task_local! {
    static HOST_PROXY: Arc<HostProxy>;
}

/// 返回当前 `run()` scope 内的 task-local `HostProxy`。
/// 在 `run()` 之外调用会 panic。
pub fn host() -> Arc<HostProxy> {
    HOST_PROXY.with(|h| h.clone())
}

/// 从 read task 路由到 dispatch task 的入站 JSON-RPC 请求。
struct IncomingRequest {
    id: u64,
    method: String,
    params: serde_json::Value,
}

/// 使用给定的 Plugin 实现运行 JSON-RPC stdio 循环。
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

    // 通道
    let (request_tx, mut request_rx) = mpsc::channel::<IncomingRequest>(64);
    let (outbound_tx, mut outbound_rx) = mpsc::channel::<Vec<u8>>(64);
    let pending: Arc<DashMap<u64, oneshot::Sender<serde_json::Value>>> = Arc::new(DashMap::new());

    // 创建 HostProxy。当 scope 退出时，HOST_PROXY 被 drop，
    // 从而释放 outbound_tx 的最后一个 clone，让 write task
    // 通过 channel 关闭优雅退出。
    let host_proxy = Arc::new(HostProxy::new(pending.clone(), outbound_tx.clone()));

    HOST_PROXY
        .scope(host_proxy, async move {
            // 插件状态
            let mut plugin_context: Option<zerolaunch_plugin_api::PluginContext> = None;

            // --- 读任务：stdin → pending_map（响应）或 request_tx（新请求）---
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
                                    } else {
                                        // 如果没有对应的 pending channel，说明响应已经超时或被取消，忽略。同时打印一下被忽略的信息
                                        tracing::warn!(
                                            "收到未知的响应 id={}，可能已超时或被取消: {:?}",
                                            resp.id,
                                            resp
                                        );
                                    }
                                }
                                Message::Request(req) => {
                                    let ret = request_tx_clone
                                        .send(IncomingRequest {
                                            id: req.id,
                                            method: req.method,
                                            params: req.params,
                                        })
                                        .await;
                                    // 如果 dispatch task 已退出，说明插件可能已经崩溃或被关闭，无法处理请求。打印警告信息。
                                    if ret.is_err() {
                                        tracing::warn!(
                                            "无法将请求发送到 dispatch task，可能 dispatch task 已退出: {:?}",
                                            ret
                                        );
                                    }
                                }
                                Message::Notification(_) => {
                                    tracing::trace!("忽略通知");
                                }
                            }
                        }
                        Err(_) => break,
                    }
                }
            });

            // --- 分发任务：plugin/* 请求 → 用户 Plugin → 响应到 outbound_tx ---
            let outbound_dispatch = outbound_tx.clone();
            let dispatch_handle = tokio::spawn(async move {
                while let Some(incoming) = request_rx.recv().await {
                    let req = Request::new(incoming.id, &incoming.method, incoming.params);
                    // 收到了一个请求，调用用户实现的 Plugin trait 处理，并将响应发送到 outbound_tx。
                    let result = handle_request(&mut plugin, &req, &mut plugin_context).await;
                    if let Ok(payload) = serde_json::to_vec(&result) {
                        if outbound_dispatch.send(payload).await.is_err() {
                            break;
                        }
                    }
                }
            });

            // --- 写任务：outbound_rx → stdout ---
            let write_handle = tokio::spawn(async move {
                let mut writer = stdout;
                while let Some(payload) = outbound_rx.recv().await {
                    let frame = encode_frame(&payload);
                    if writer.write_all(&frame).await.is_err() {
                        break;
                    }
                    if writer.flush().await.is_err() {
                        break;
                    }
                }
            });

            // 等待读任务结束（传输层关闭）。
            let _ = read_handle.await;

            // 释放 request_tx → dispatch_task 在当前请求处理完后
            // 通过 channel 关闭优雅退出。
            drop(request_tx);

            // 写任务无法通过 channel 关闭退出，因为
            // HOST_PROXY（在此 scope 内）仍持有
            // outbound_tx 的 clone，因此直接 abort。
            write_handle.abort();

            let _ = tokio::join!(dispatch_handle, write_handle);
        })
        .await;
    // HOST_PROXY scope 在此结束 → Arc<HostProxy> 释放 → 最终清理。
}

/// 读取单条 LSP 风格 Content-Length 帧消息。
/// 返回原始 JSON 字节，或在解析/大小/IO 失败时返回错误字符串。
async fn read_frame<R: tokio::io::AsyncBufRead + Unpin>(reader: &mut R) -> Result<Vec<u8>, String> {
    let mut content_length: Option<usize> = None;
    let mut total_header_len = 0usize;
    loop {
        let mut line = String::new();
        let n = reader
            .read_line(&mut line)
            .await
            .map_err(|e| format!("read error: {}", e))?;
        if n == 0 {
            return Err("transport closed".into());
        }
        total_header_len += n;
        if total_header_len > MAX_HEADER_SIZE {
            return Err("header too long".into());
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            break;
        }
        if let Some(value) = trimmed.strip_prefix("Content-Length:") {
            content_length = Some(
                value
                    .trim()
                    .parse::<usize>()
                    .map_err(|e| format!("bad Content-Length: {}", e))?,
            );
        }
    }
    let len = content_length.ok_or("missing Content-Length")?;
    if len > MAX_FRAME_SIZE {
        return Err(format!("Content-Length too large: {}", len));
    }
    let mut body = vec![0u8; len];
    reader
        .read_exact(&mut body)
        .await
        .map_err(|e| format!("read body: {}", e))?;
    Ok(body)
}

// / 处理单条 plugin/* 请求，返回响应 Message。
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
        // 初始化请求，设置 plugin_ctx 并返回插件版本信息。
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
        // todo： 要有具体的处理方法，不可以直接这样返回，比如真的结束这个进程
        plugin_methods::SHUTDOWN => Ok(serde_json::Value::Null),
        // 返回 metadata
        plugin_methods::GET_METADATA => {
            Ok(serde_json::to_value(plugin.metadata()).unwrap_or(serde_json::Value::Null))
        }
        // 返回这个插件实现的components是什么
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
        // 返回注册的配置项
        plugin_methods::GET_SETTINGS_SCHEMA => {
            Ok(serde_json::to_value(plugin.setting_schema()).unwrap_or(serde_json::Value::Null))
        }
        // 返回插件当前的配置值
        plugin_methods::GET_SETTINGS => Ok(plugin.get_settings()),
        // 宿主下发新的配置值，插件据此更新自身行为
        plugin_methods::APPLY_SETTINGS => {
            let p: ApplySettingsParams = serde_json::from_value(params.clone())
                .map_err(|e| JsonRpcError::new(codes::INVALID_PARAMS, e.to_string()))?;
            plugin
                .apply_settings(p.settings)
                .map_err(|e| JsonRpcError::new(codes::PLUGIN_ERROR, e.to_string()))?;
            Ok(serde_json::Value::Null)
        }
        // 验证一组配置值是否合法（不会实际应用），返回验证结果或错误信息
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
