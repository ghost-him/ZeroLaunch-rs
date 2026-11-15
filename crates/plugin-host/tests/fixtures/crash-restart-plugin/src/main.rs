//! 崩溃重启集成测试插件（fixture）的可执行体。
//!
//! 本二进制是 `tests/fixtures/crash-restart-plugin/manifest.toml` 声明的
//! runtime.command 产物：以 LSP Content-Length 帧在 stdio 上响应宿主的
//! plugin/* RPC，声明一个 Plugin 组件（无 schema/settings/actions）。
//! 测试运行时将编译产物复制进插件目录的 bin/（见 tests/crash_restart.rs），
//! 不作为产品二进制发布。

use std::io::{BufRead, Read, Write};
use std::sync::Mutex;

use zerolaunch_plugin_protocol::codec::encode_frame;
use zerolaunch_plugin_protocol::error::JsonRpcError;
use zerolaunch_plugin_protocol::jsonrpc::{Request, Response};
use zerolaunch_plugin_protocol::methods::plugin as plugin_methods;
use zerolaunch_plugin_protocol::PROTOCOL_VERSION;

/// 宿主 `plugin/initialize` 下发的插件 id（Plugin 组件 id 必须与它相等）。
static PLUGIN_ID: Mutex<Option<String>> = Mutex::new(None);
/// 额外声明的 DataSource 组件 id（跨插件固定，用于构造组件 id 冲突场景）。
pub const FIXTURE_SHARED_COMPONENT_ID: &str = "fixture.shared";

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
        plugin_methods::INITIALIZE => {
            if let Some(id) = request.params.get("pluginId").and_then(|v| v.as_str()) {
                *PLUGIN_ID.lock().expect("plugin id lock") = Some(id.to_string());
            }
            serde_json::json!({
                "protocolVersion": PROTOCOL_VERSION,
            })
        }
        plugin_methods::GET_COMPONENTS => {
            let plugin_id = PLUGIN_ID
                .lock()
                .ok()
                .and_then(|id| id.clone())
                .unwrap_or_default();
            // 以 -mismatch 结尾的插件返回不等的组件 id，用于锚点校验负向测试。
            let plugin_component_id = if plugin_id.ends_with("-mismatch") {
                format!("{plugin_id}.wrong")
            } else {
                plugin_id
            };
            serde_json::json!([
                {
                    "componentId": plugin_component_id,
                    "componentName": "Fixture Hello",
                    "componentDescription": "crash restart integration test fixture",
                    "kind": { "type": "plugin" },
                    "priority": 50,
                },
                {
                    "componentId": FIXTURE_SHARED_COMPONENT_ID,
                    "componentName": "Fixture Shared Source",
                    "componentDescription": "shared data source for component id collision tests",
                    "kind": { "type": "data_source" },
                    "priority": 50,
                }
            ])
        }
        // Plugin 组件无配置项：schema/settings/actions 一律返回空
        plugin_methods::GET_SETTINGS_SCHEMA => serde_json::json!([]),
        plugin_methods::GET_SETTINGS => serde_json::json!({}),
        plugin_methods::CONFIG_ACTIONS => serde_json::json!([]),
        // 退出钩子：每次收到 plugin/shutdown 都向数据目录追加一行，供集成测试
        // 断言"先于进程退出到达"且"恰好一次"；响应体为空（宿主不消费）。
        plugin_methods::SHUTDOWN => {
            if let Ok(dir) = std::env::var("ZEROLAUNCH_DATA_DIR") {
                if let Ok(mut file) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(std::path::Path::new(&dir).join("shutdown-hook"))
                {
                    let _ = writeln!(file, "shutdown");
                }
            }
            serde_json::Value::Null
        }
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
