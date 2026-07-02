# ZeroLaunch 第三方插件开发指南

## 总览

第三方插件以**独立子进程**方式运行，通过 **stdio JSON-RPC 2.0** 与 ZeroLaunch 宿主通信。
支持任意编程语言（Rust、Python、Node.js 等）。

### 插件能做什么

- **Plugin（触发式）**：定义触发关键词，用户输入时返回搜索结果或自定义面板
- **DataSource**：提供搜索候选项（如列出密码条目、书签）
- **ActionExecutor**：为特定 TargetType 注册执行动作（如"用 IDA 打开"）

### 插件不能做什么（第一版）

- 修改搜索引擎算法
- 影响分数排序
- 访问其他插件的数据

## 快速开始（Rust）

### 1. 创建项目

```toml
# Cargo.toml
[package]
name = "my-plugin"
version = "0.1.0"
edition = "2021"

[dependencies]
zerolaunch-plugin-sdk-rust = { path = "../ZeroLaunch-rs/crates/plugin-sdk-rust" }
zerolaunch-plugin-api = { path = "../ZeroLaunch-rs/crates/plugin-api" }
zerolaunch-plugin-protocol = { path = "../ZeroLaunch-rs/crates/plugin-protocol" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
async-trait = "0.1"
```

### 2. 实现 Plugin trait

```rust
use zerolaunch_plugin_sdk_rust::run;
use zerolaunch_plugin_api::*;

struct MyPlugin;

#[async_trait::async_trait]
impl Plugin for MyPlugin {
    fn metadata(&self) -> &PluginMetadata { /* ... */ }
    async fn init(&self, ctx: &PluginContext, handle: Arc<PluginHandle>) -> Result<(), PluginError> { Ok(()) }
    async fn query(&self, ctx: &PluginContext, query: &Query) -> Result<QueryResponse, PluginError> { /* ... */ }
    async fn execute_action(&self, ctx: &PluginContext, action_id: &str, payload: serde_json::Value) -> Result<(), PluginError> { /* ... */ }
}

fn main() {
    run(MyPlugin)
}
```

### 3. 编写 manifest.toml

```toml
[plugin]
id = "com.example.my-plugin"
name = "我的插件"
version = "0.1.0"
description = "插件描述"
author = "Your Name <you@example.com>"
min_host_version = "0.7.0"

[runtime]
command = "target/release/my-plugin.exe"

[components]
provides = ["plugin"]
```

### 4. 安装

将编译产物和 manifest.toml 放入：
```
%APPDATA%/ZeroLaunch/plugins/com.example.my-plugin/
├── manifest.toml
└── bin/plugin.exe
```

## Python 插件开发

Python 插件直接读写 stdin/stdout，遵循 LSP-style Content-Length 帧格式：

```python
import sys, json

def read_message():
    headers = {}
    while True:
        line = sys.stdin.readline().strip()
        if not line: break
        key, val = line.split(":", 1)
        headers[key.strip()] = val.strip()
    length = int(headers["Content-Length"])
    body = sys.stdin.read(length)
    return json.loads(body)

def send_response(id, result):
    payload = json.dumps({"jsonrpc": "2.0", "id": id, "result": result})
    header = f"Content-Length: {len(payload)}\r\n\r\n"
    sys.stdout.write(header + payload)
    sys.stdout.flush()

while True:
    msg = read_message()
    method = msg["method"]
    # Handle initialize, query, execute_action, etc.
```

## 前端 UI

如果 manifest.toml 中包含 `[ui]` 部分，宿主会通过 `zlplugin://` 协议暴露 UI 资源：

```toml
[ui]
panel_entry = "ui/panel.mjs"
```

`panel.mjs` 必须 export default function：

```js
export default function mount(rootEl, host) {
  rootEl.innerHTML = '<div>Hello</div>'
  host.onDataUpdate((data, actions) => {
    // 收到宿主的更新数据
  })
}
```

## 调试

- 查看日志：`%APPDATA%/ZeroLaunch/plugin-logs/<plugin-id>.log`
- 使用 Plugin Inspector（设置 → 插件检查器）
- stderr 输出会被自动收集
