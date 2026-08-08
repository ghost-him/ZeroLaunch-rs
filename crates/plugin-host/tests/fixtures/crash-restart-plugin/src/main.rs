//! 崩溃重启集成测试插件（fixture）的可执行体。
//!
//! 本二进制是 `tests/fixtures/crash-restart-plugin/manifest.toml` 声明的
//! runtime.command 产物：以 LSP Content-Length 帧在 stdio 上响应宿主的
//! plugin/* RPC，声明一个 Plugin 组件（无 schema/settings/actions）。
//! 测试运行时将编译产物复制进插件目录的 bin/（见 tests/crash_restart.rs），
//! 不作为产品二进制发布。

use std::io::{BufRead, Read, Write};

use zerolaunch_plugin_protocol::codec::encode_frame;
use zerolaunch_plugin_protocol::error::JsonRpcError;
use zerolaunch_plugin_protocol::jsonrpc::{Request, Response};
use zerolaunch_plugin_protocol::methods::plugin as plugin_methods;
use zerolaunch_plugin_protocol::PROTOCOL_VERSION;

/// 本 fixture 声明的组件 id（测试依赖它构造冲突场景）。
pub const FIXTURE_COMPONENT_ID: &str = "fixture.hello";

fn main() {
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    let stdout = std::io::stdout();
    let mut output = stdout.lock();

    loop {
        // 读取帧头（Content-Length: N\r\n\r\n）
        let mut content_length = None;
        loop {
            let mut line = String::new();
            match input.read_line(&mut line) {
                Ok(0) => return, // 宿主关闭 stdin（进程即将被终止）
                Ok(_) => {}
                Err(_) => return,
            }
            let trimmed = line.trim_end();
            if trimmed.is_empty() {
                break;
            }
            if let Some(value) = trimmed
                .strip_prefix("Content-Length:")
                .and_then(|v| v.trim().parse::<usize>().ok())
            {
                content_length = Some(value);
            }
        }
        let Some(len) = content_length else { return };

        // 读取请求体并解析 JSON-RPC 请求
        let mut body = vec![0u8; len];
        if input.read_exact(&mut body).is_err() {
            return;
        }
        let request: Request = match serde_json::from_slice(&body) {
            Ok(req) => req,
            Err(_) => continue,
        };

        let response = handle(&request);
        let payload = serde_json::to_vec(&response).expect("response serializable");
        let frame = encode_frame(&payload);
        if output.write_all(&frame).is_err() {
            return;
        }
        let _ = output.flush();
    }
}

/// 按请求方法构造响应：固定返回一个 Plugin 组件与空 schema/settings/actions。
fn handle(request: &Request) -> Response {
    let result = match request.method.as_str() {
        plugin_methods::INITIALIZE => serde_json::json!({
            "pluginVersion": "1.0.0",
            "protocolVersion": PROTOCOL_VERSION,
        }),
        plugin_methods::GET_METADATA => {
            let plugin_id =
                std::env::var("ZEROLAUNCH_PLUGIN_ID").unwrap_or_else(|_| "fixture".to_string());
            serde_json::json!({
                "id": plugin_id,
                "name": "Crash Restart Fixture",
                "version": "1.0.0",
                "description": "crash restart integration test fixture",
                "author": "zerolaunch",
                "triggerKeywords": [],
                "supportedOs": ["windows"],
                "priority": 50,
            })
        }
        plugin_methods::GET_COMPONENTS => serde_json::json!([{
            "componentId": FIXTURE_COMPONENT_ID,
            "componentName": "Fixture Hello",
            "componentDescription": "crash restart integration test fixture",
            "componentType": "Plugin",
            "kind": { "type": "plugin", "triggerKeywords": [] },
            "priority": 50,
        }]),
        // Plugin 组件无配置项：schema/settings/actions 一律返回空
        plugin_methods::GET_SETTINGS_SCHEMA => serde_json::json!([]),
        plugin_methods::GET_SETTINGS => serde_json::json!({}),
        plugin_methods::CONFIG_ACTIONS => serde_json::json!([]),
        other => {
            return Response::err(
                request.id,
                JsonRpcError::new(
                    zerolaunch_plugin_protocol::codes::METHOD_NOT_FOUND,
                    format!("fixture: unknown method {}", other),
                ),
            );
        }
    };
    Response::ok(request.id, result)
}
