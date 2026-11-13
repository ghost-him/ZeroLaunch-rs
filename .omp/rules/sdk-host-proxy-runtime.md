---
description: 第三方插件 SDK host() 代理与进程退出 — host() 经 OnceLock 代理、run() 前调用 panic，stdin EOF 是唯一退出路径
condition: "HOST_PROXY|fn host\\(|run_async|stdin|EOF|select!|HostProxy|host_proxy"
scope: "tool:read(crates/plugin-sdk-rust/**), tool:edit(crates/plugin-sdk-rust/**), tool:write(crates/plugin-sdk-rust/**), tool:read(crates/plugin-host/**), tool:edit(crates/plugin-host/**), tool:write(crates/plugin-host/**), tool:read(crates/zerolaunch-rs/src/plugin_framework/host_handler.rs), tool:edit(crates/zerolaunch-rs/src/plugin_framework/host_handler.rs), tool:write(crates/zerolaunch-rs/src/plugin_framework/host_handler.rs)"
---

# SDK host() 代理与进程退出契约

## host() 代理

- SDK 用 `HOST_PROXY: OnceLock<HostProxy>` 存宿主代理，`host()` 返回其引用；在 `run()` 之前调用 `host()` **panic**（代理需握手后注入）
- `HostProxy` 暴露宿主能力方法：`model_list`/`model_chat`/`model_embedding`/`model_similarity`/`get_theme`/`get_locale` 等，内部走 JSON-RPC 到宿主 `host/*` 端点

## 关键陷阱

- **task_local 不跨 `tokio::spawn` 继承**：宿主代理若用 `tokio::task_local!` 传递，dispatch task 内 `host()` 必 panic（`cannot access a task-local storage value`）→ 表现为插件 RPC 超时（如 `Configuration apply failed: timeout`）。修复方案是 `OnceLock` 全局代理，禁止回到 task_local 传递
- `host()` 只能在 `run()` 之后、插件 dispatch 回调内调用

## 进程退出

- **stdin EOF 是 SDK 进程的唯一退出路径**：宿主关闭 stdin（transport close）→ SDK `run_async` 的 `select!` 收到 EOF → 进程退出。协议层无 shutdown RPC
- 宿主 `PluginProcess::shutdown`：标记 Stopped → 关 stdin → 等 watchdog → 超时强杀（taskkill）兜底
- 插件侧：stdin EOF 后不得继续持有阻塞资源，应立即清理退出；拖沓会让宿主走强杀路径
