---
description: 禁止将 AppState 放入全局静态变量 — 必须通过状态注入或 Host 端口获取
condition: "(lazy_static!|once_cell::sync::Lazy|std::sync::OnceLock|static\\s+(ref\\s+)?mut).*AppState"
scope: "tool:edit(*.rs), tool:write(*.rs)"
---

你正试图将 `Arc<AppState>` 放入全局静态变量中。本项目严禁通过全局静态访问 AppState。

正确的 AppState 访问方式：
- `commands/` 层：通过 `tauri::State<Arc<AppState>>` 参数注入（Tauri 外壳期）
- 回调闭包：在函数开始时取 `Arc<AppState>` 或 `Host` 的 clone，再用 `move` 闭包捕获
- 其他模块：通过参数注入 `&AppState` / `Arc<AppState>`；窗口与事件出口使用 `AppState::get_host()`
- 内核装配完成后，宿主实现由 `AppState::set_host()` 安装，禁止回读 Tauri `AppHandle`

示例（正确）：
```rust
#[tauri::command]
fn my_command(state: tauri::State<Arc<AppState>>) -> Result<(), BridgeError> {
    let config = state.config.read();
    // ...
}
```

示例（无窗口场景）：
```rust
let host = state.get_host();
host.notify("ZeroLaunch", "message");
```
