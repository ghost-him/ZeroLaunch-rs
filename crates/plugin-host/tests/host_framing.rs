//! 宿主侧 JSON-RPC 帧协议的集成测试。
//!
//! 覆盖真实协议链路的关键契约（与双重分帧 bug 相关）：
//!
//! 1. 宿主能正确解析插件发来的**单层分帧** host/* 请求
//!    （`Content-Length: N\r\n\r\n{JSON}`），并路由到 incoming channel；
//! 2. 宿主对 host/* 请求的响应（respond_ok）能写回插件侧并可解析；
//! 3. 宿主→插件的请求（call）是单层分帧，插件侧可解析并回响应；
//! 4. 插件发来的**双重分帧**消息（修复前 SDK 的 bug：
//!    `Content-Length: M\r\n\r\nContent-Length: N\r\n\r\n{JSON}`）
//!    必须被宿主识别为损坏帧并丢弃——回归测试的"对立面"。
//!
//! 配套的 SDK 侧单测（plugin-sdk-rust/src/host_proxy.rs）验证插件发送端
//! 只投递干净 JSON；本测试验证宿主接收端按单层分帧解析。

use std::time::Duration;

use tokio::io::{AsyncWriteExt, BufReader, BufWriter, DuplexStream};
use tokio::sync::mpsc;

use zerolaunch_plugin_host::client::{IncomingRequest, JsonRpcClient};
use zerolaunch_plugin_host::transport::codec;
use zerolaunch_plugin_protocol::codec::encode_frame;
use zerolaunch_plugin_protocol::jsonrpc::{Message, Response};

/// 构造两对连通的内存管道，分别承载两个方向：
/// - (plugin→host)：插件写端 + 宿主读端
/// - (host→plugin)：宿主写端 + 插件读端
struct Harness {
    plugin_to_host_writer: DuplexStream,
    host_reader: DuplexStream,
    host_writer: DuplexStream,
    host_to_plugin_reader: DuplexStream,
}

/// 拆解 harness：宿主侧 client 需要宿主读/写两端，
/// 插件侧仍持有插件写/读两端用于模拟插件行为。
fn split_harness(h: Harness) -> ((DuplexStream, DuplexStream), (DuplexStream, DuplexStream)) {
    (
        (h.host_reader, h.host_writer),
        (h.plugin_to_host_writer, h.host_to_plugin_reader),
    )
}

fn harness() -> Harness {
    let (p2h_w, p2h_r) = tokio::io::duplex(64 * 1024);
    let (h2p_w, h2p_r) = tokio::io::duplex(64 * 1024);
    Harness {
        plugin_to_host_writer: p2h_w,
        host_reader: p2h_r,
        host_writer: h2p_w,
        host_to_plugin_reader: h2p_r,
    }
}

/// 宿主侧：创建 JsonRpcClient，并返回收到 host/* 请求的通道。
fn host_client(
    host_reader: DuplexStream,
    host_writer: DuplexStream,
) -> (
    std::sync::Arc<JsonRpcClient>,
    mpsc::Receiver<IncomingRequest>,
) {
    let (req_tx, req_rx) = mpsc::channel::<IncomingRequest>(16);
    let client = JsonRpcClient::new(BufReader::new(host_reader), host_writer, req_tx);
    (client, req_rx)
}

/// 场景 1：插件发送单层分帧的 host/* 请求，宿主能解析、路由并回响应。
#[tokio::test]
async fn host_parses_single_framed_plugin_request_and_responds() {
    let ((host_reader, host_writer), (plugin_writer, plugin_reader)) = split_harness(harness());
    let (client, mut req_rx) = host_client(host_reader, host_writer);

    // 插件侧：发送 host/log 请求（单层分帧，修复后 SDK 的正确行为）
    let mut writer = plugin_writer;
    let payload =
        br#"{"jsonrpc":"2.0","id":1,"method":"host/log","params":{"level":"warn","message":"hi"}}"#;
    let frame = encode_frame(payload);
    writer.write_all(&frame).await.expect("write frame");
    writer.flush().await.expect("flush");

    // 宿主应收到该请求
    let incoming = tokio::time::timeout(Duration::from_secs(3), req_rx.recv())
        .await
        .expect("宿主应在超时内收到插件请求")
        .expect("channel 不应关闭");
    assert_eq!(incoming.id, 1);
    assert_eq!(incoming.method, "host/log");
    assert_eq!(incoming.params["level"], "warn");

    // 宿主回响应，插件侧应能读到（单层分帧、可解析）
    client
        .respond_ok(1, serde_json::json!({ "ok": true }))
        .await
        .expect("respond_ok 成功");

    // 插件侧解析宿主响应（用宿主 codec 的 read_frame）
    let mut plugin_reader = BufReader::new(plugin_reader);
    let frame = codec::read_frame(&mut plugin_reader)
        .await
        .expect("宿主响应应为合法单层分帧");
    let msg: Message = serde_json::from_slice(&frame).expect("宿主响应应为合法 JSON");
    match msg {
        Message::Response(resp) => {
            assert_eq!(resp.id, 1);
            assert_eq!(resp.result, Some(serde_json::json!({ "ok": true })));
        }
        other => panic!("宿主应返回 Response，实际: {:?}", other),
    }
}

/// 场景 2：插件发送双重分帧（修复前 SDK bug 的产物），
/// 宿主必须将其识别为损坏帧并丢弃（不能 panic，不能误路由）。
#[tokio::test]
async fn host_rejects_double_framed_plugin_message() {
    let ((host_reader, host_writer), (plugin_writer, _plugin_reader)) = split_harness(harness());
    let (_client, mut req_rx) = host_client(host_reader, host_writer);

    // 双重分帧：payload 本身是一个完整的帧字符串
    // （修复前 HostProxy 先 encode_frame，write_task 再 encode_frame 的产物）
    let inner_payload =
        br#"{"jsonrpc":"2.0","id":1,"method":"host/log","params":{"level":"warn","message":"hi"}}"#;
    let inner_frame = encode_frame(inner_payload);
    let outer_frame = encode_frame(&inner_frame); // 再套一层帧头

    let mut writer = plugin_writer;
    writer.write_all(&outer_frame).await.expect("write frame");
    writer.flush().await.expect("flush");

    // 宿主 read_frame 得到 body = "Content-Length: N\r\n\r\n{...}"，
    // serde_json 解析失败 → 该帧被丢弃。验证：短时间内不应收到任何请求。
    let received = tokio::time::timeout(Duration::from_millis(800), req_rx.recv()).await;
    assert!(
        received.is_err(),
        "双重分帧的损坏帧必须被宿主丢弃，不得路由: {:?}",
        received
    );
}

/// 场景 3：宿主→插件的请求是单层分帧（宿主写 stdin 侧正确），
/// 且插件回响应后宿主的 call 能拿到结果——完整 RPC 往返。
#[tokio::test]
async fn host_request_plugin_responds_full_roundtrip() {
    let ((host_reader, host_writer), (plugin_writer, plugin_reader)) = split_harness(harness());
    let (client, _req_rx) = host_client(host_reader, host_writer);

    // 宿主调用插件方法（如 plugin/get_settings）
    let call = tokio::spawn(async move {
        client
            .call::<_, serde_json::Value>(
                "plugin/get_settings",
                serde_json::json!({ "componentId": "x" }),
                Duration::from_secs(3),
            )
            .await
    });

    // 插件侧读宿主请求：必须是单层分帧的合法 JSON
    let mut plugin_reader = BufReader::new(plugin_reader);
    let frame = codec::read_frame(&mut plugin_reader)
        .await
        .expect("宿主请求应为合法单层分帧");
    let msg: Message = serde_json::from_slice(&frame).expect("宿主请求应为合法 JSON");
    let Message::Request(req) = msg else {
        panic!("宿主应发送 Request，实际: {:?}", msg);
    };
    assert_eq!(req.method, "plugin/get_settings");

    // 插件侧回响应（单层分帧）
    let resp_payload =
        serde_json::to_vec(&Response::ok(req.id, serde_json::json!({ "a": 1 }))).unwrap();
    let mut writer = BufWriter::new(plugin_writer);
    writer
        .write_all(&encode_frame(&resp_payload))
        .await
        .unwrap();
    writer.flush().await.unwrap();

    // 宿主的 call 应拿到结果
    let result = tokio::time::timeout(Duration::from_secs(3), call)
        .await
        .expect("call 不应挂起")
        .expect("任务不 panic");
    assert_eq!(result.expect("RPC 成功"), serde_json::json!({ "a": 1 }));
}
