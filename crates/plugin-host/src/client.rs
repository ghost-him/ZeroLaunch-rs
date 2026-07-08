use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::io::{AsyncBufRead, AsyncWrite};
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error, warn};

use zerolaunch_plugin_protocol::{codes, JsonRpcError, Message, ProtocolError, Request, Response};

use crate::transport::codec;

/// Incoming request from a plugin subprocess.
///
/// Responses are sent via `JsonRpcClient::respond_ok` / `respond_err`,
/// which write through the outbound channel.
pub struct IncomingRequest {
    pub id: u64,
    pub method: String,
    pub params: serde_json::Value,
}

/// Bidirectional JSON-RPC 2.0 client over a framed stdio transport.
#[derive(Debug)]
pub struct JsonRpcClient {
    next_id: AtomicU64,
    pending: Arc<DashMap<u64, oneshot::Sender<Result<serde_json::Value, JsonRpcError>>>>,
    outbound_tx: mpsc::Sender<Message>,
    /// Handle to the read-loop task.
    _read_handle: tokio::task::JoinHandle<()>,
    /// Handle to the write-loop task.
    _write_handle: tokio::task::JoinHandle<()>,
}

impl JsonRpcClient {
    /// Creates a new client, spawning background read/write loops.
    ///
    /// - `reader` / `writer`: the stdio handles from the child process.
    /// - `incoming_request_tx`: channel for forwarding plugin→host requests.
    /// - `incoming_notification_tx`: channel for forwarding plugin→host notifications.
    /// - Returns an `Arc<JsonRpcClient>` so it can be shared with host_dispatch.
    pub fn new<R, W>(
        reader: R,
        writer: W,
        incoming_request_tx: mpsc::Sender<IncomingRequest>,
        incoming_notification_tx: mpsc::Sender<(String, serde_json::Value)>,
    ) -> Arc<Self>
    where
        R: AsyncBufRead + Send + Unpin + 'static,
        W: AsyncWrite + Send + Unpin + 'static,
    {
        let pending: Arc<DashMap<u64, oneshot::Sender<Result<serde_json::Value, JsonRpcError>>>> =
            Arc::new(DashMap::new());

        // 串行写入：防止多线程并发写入导致 JSON-RPC 帧交错
        let (outbound_tx, outbound_rx) = mpsc::channel::<Message>(256);

        // Read loop
        let pending_r = pending.clone();
        let read_handle = tokio::spawn(async move {
            let mut reader = reader;
            loop {
                let frame = match codec::read_frame(&mut reader).await {
                    Ok(f) => f,
                    Err(ProtocolError::TransportClosed) => {
                        debug!("Transport closed (read loop)");
                        break;
                    }
                    Err(e) => {
                        error!("Frame read error: {}", e);
                        break;
                    }
                };

                let msg: Message = match serde_json::from_slice(&frame) {
                    Ok(m) => m,
                    Err(e) => {
                        warn!("Failed to parse JSON-RPC message: {}", e);
                        continue;
                    }
                };

                match msg {
                    Message::Response(resp) => {
                        if let Some((_, tx)) = pending_r.remove(&resp.id) {
                            let result = if let Some(err) = resp.error {
                                Err(err)
                            } else {
                                Ok(resp.result.unwrap_or(serde_json::Value::Null))
                            };
                            let _ = tx.send(result);
                        } else {
                            // 正常不应该走到这
                            debug!("Response for unknown id: {}", resp.id);
                        }
                    }
                    Message::Request(req) => {
                        let incoming = IncomingRequest {
                            id: req.id,
                            method: req.method,
                            params: req.params,
                        };
                        if incoming_request_tx.send(incoming).await.is_err() {
                            debug!("Incoming request channel closed");
                        }
                    }
                    Message::Notification(notif) => {
                        if incoming_notification_tx
                            .send((notif.method, notif.params))
                            .await
                            .is_err()
                        {
                            debug!("Incoming notification channel closed");
                        }
                    }
                }
            }

            // Clean up all pending requests on transport close
            let keys: Vec<u64> = pending_r.iter().map(|e| *e.key()).collect();
            for key in keys {
                if let Some((_, tx)) = pending_r.remove(&key) {
                    let _ = tx.send(Err(JsonRpcError::new(
                        codes::PLUGIN_CRASHED,
                        "transport closed",
                    )));
                }
            }
        });

        // Write loop
        let write_handle = tokio::spawn(async move {
            let mut writer = writer;
            let mut rx = outbound_rx;
            while let Some(msg) = rx.recv().await {
                let payload = match serde_json::to_vec(&msg) {
                    Ok(p) => p,
                    Err(e) => {
                        error!("Failed to serialize message: {}", e);
                        continue;
                    }
                };
                if let Err(e) = codec::write_frame(&mut writer, &payload).await {
                    error!("Failed to write frame: {}", e);
                    break;
                }
            }
        });

        Arc::new(Self {
            next_id: AtomicU64::new(1),
            pending,
            outbound_tx,
            _read_handle: read_handle,
            _write_handle: write_handle,
        })
    }

    /// Send a request and wait for the response.
    pub async fn call<P: Serialize, R: DeserializeOwned>(
        &self,
        method: &str,
        params: P,
        timeout: Duration,
    ) -> Result<R, ProtocolError> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let params_value = serde_json::to_value(params)?;

        let (tx, rx) = oneshot::channel();
        self.pending.insert(id, tx);

        let request = Request::new(id, method, params_value);
        self.outbound_tx
            .send(Message::Request(request))
            .await
            .map_err(|_| ProtocolError::TransportClosed)?;

        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(Ok(value))) => {
                let result: R = serde_json::from_value(value)?;
                Ok(result)
            }
            Ok(Ok(Err(err))) => Err(ProtocolError::Rpc {
                code: err.code,
                message: err.message,
            }),
            Ok(Err(_)) => {
                self.pending.remove(&id);
                Err(ProtocolError::TransportClosed)
            }
            Err(_) => {
                self.pending.remove(&id);
                Err(ProtocolError::Timeout)
            }
        }
    }

    /// Send a notification (fire-and-forget, no response expected).
    pub async fn notify<P: Serialize>(&self, method: &str, params: P) -> Result<(), ProtocolError> {
        let params_value = serde_json::to_value(params)?;
        let notif = zerolaunch_plugin_protocol::Notification::new(method, params_value);
        self.outbound_tx
            .send(Message::Notification(notif))
            .await
            .map_err(|_| ProtocolError::TransportClosed)?;
        Ok(())
    }

    /// Send a success response to a previously received request.
    pub async fn respond_ok(
        &self,
        id: u64,
        result: serde_json::Value,
    ) -> Result<(), ProtocolError> {
        let response = Response::ok(id, result);
        self.outbound_tx
            .send(Message::Response(response))
            .await
            .map_err(|_| ProtocolError::TransportClosed)?;
        Ok(())
    }

    /// Send an error response to a previously received request.
    pub async fn respond_err(&self, id: u64, error: JsonRpcError) -> Result<(), ProtocolError> {
        let response = Response::err(id, error);
        self.outbound_tx
            .send(Message::Response(response))
            .await
            .map_err(|_| ProtocolError::TransportClosed)?;
        Ok(())
    }
}
