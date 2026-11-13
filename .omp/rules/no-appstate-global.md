---
description: 禁止将 AppState 放入全局静态变量 — 必须通过构造参数或 Host 端口获取
condition: "(lazy_static!|once_cell::sync::Lazy|std::sync::OnceLock|static\\s+(ref\\s+)?mut).*AppState"
scope: "tool:edit(*.rs), tool:write(*.rs)"
---

你正试图将 `Arc<AppState>` 放入全局静态变量。本项目严禁通过全局静态访问 AppState。

正确方式：

- GPUI 视图：创建时注入 `Arc<AppState>`，渲染与事件处理从 `self.state` 读取。
- 回调闭包：函数开始时 clone `Arc<AppState>` 或 `Host`，再用 `move` 捕获。
- 其他模块：通过参数注入 `&AppState` / `Arc<AppState>`；窗口与事件出口使用 `AppState::get_host()`。
- 宿主实现由 `AppState::set_host()` 安装；禁止回读任何 UI 框架句柄。

```rust
let state = self.state.clone();
let host = state.get_host();
host.notify("ZeroLaunch", "message");
```
