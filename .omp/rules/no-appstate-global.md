---
description: 禁止将 AppState 放入全局静态变量 — 必须通过 Tauri state 机制获取
condition: "(lazy_static!|once_cell::sync::Lazy|std::sync::OnceLock|static\\s+(ref\\s+)?mut).*AppState"
scope: "tool:edit(*.rs), tool:write(*.rs)"
---

你正试图将 `Arc<AppState>` 放入全局静态变量中。本项目严禁通过全局静态访问 AppState。

正确的 AppState 访问方式：
- `commands/` 层：通过 `tauri::State<Arc<AppState>>` 参数注入
- 回调闭包：在函数开始时通过 `app_handle.state::<Arc<AppState>>()` 获取，再用 `move` 闭包捕获 clone
- 其他模块：从 Tauri `AppHandle` 中获取，禁止全局静态

示例（正确）：
```rust
#[tauri::command]
fn my_command(state: tauri::State<Arc<AppState>>) -> Result<(), BridgeError> {
    let config = state.config.read();
    // ...
}
```
