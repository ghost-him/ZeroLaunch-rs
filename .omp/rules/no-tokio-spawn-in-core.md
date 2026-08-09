---
name: no-tokio-spawn-in-core
description: src-tauri 核心程序禁止使用 tokio 协程创建函数（spawn 系列），必须使用 tauri::async_runtime 的统一入口
condition:
  - 'tokio::(task::)?spawn\b'
  - 'tokio::(task::)?spawn_blocking'
  - 'tokio::(task::)?spawn_local'
  - '\btask::spawn'
  - 'use tokio::task'
scope:
  - tool:edit(src-tauri/src/**/*.rs)
  - tool:write(src-tauri/src/**/*.rs)
  - text
---

# src-tauri 核心程序禁止 tokio 协程创建，统一使用 tauri::async_runtime

## 规则

在 `src-tauri/src/` 下（主程序核心），**禁止**直接调用 tokio 的协程创建函数：

```rust
// ❌ 错误：绕过 tauri 的任务生命周期管理
tokio::spawn(async move { ... });
tokio::task::spawn(async move { ... });
tokio::spawn_blocking(|| { ... });
use tokio::task; task::spawn(async move { ... });
```

**必须**使用 tauri 的统一异步入口：

```rust
// ✅ 正确：tauri::async_runtime 内部即 tokio，能力等价且生命周期由 tauri 管理
tauri::async_runtime::spawn(async move { ... });
tauri::async_runtime::spawn_blocking(|| { ... });
```

## 原因

- 主程序运行在 tauri 管理的 async runtime 内（`tauri::async_runtime` 内部即 tokio，能力完全等价）。
- 直接 `tokio::spawn` 绕过 tauri 的任务生命周期管理（关闭时 JoinHandle 清理、AppHandle 引用计数、runtime 优雅退出），且并发入口不统一，后续维护无法全局追踪。
- 统一走 `tauri::async_runtime` 后，任务入口单一、与 tauri 事件/窗口/托盘生命周期对齐。

## 范围边界

- **仅约束 `src-tauri/src/`**：`crates/`（plugin-host、plugin-sdk-rust、plugin-protocol 等库）无 tauri 依赖，**必须**使用 tokio 原生 API，不在本规则范围。
- **`#[tokio::test]` 测试宏不在此列**：它是测试运行时宏而非协程创建函数，测试模块中可用。
- **`tokio::time::interval` / `tokio::time::sleep` 等挂起 API 不在此列**：它们在 tauri 的 tokio runtime 下语义等价，不是协程创建函数。
