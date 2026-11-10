---
name: no-tokio-spawn-in-core
description: 宿主 crate（src-tauri / app）禁止直接创建 tokio 协程，统一使用进程级 runtime 入口
condition:
  - 'tokio::(task::)?spawn\b'
  - 'tokio::(task::)?spawn_blocking'
  - 'tokio::(task::)?spawn_local'
  - '\btask::spawn'
  - 'use tokio::task'
scope:
  - tool:edit(src-tauri/src/**/*.rs)
  - tool:write(src-tauri/src/**/*.rs)
  - tool:edit(app/src/**/*.rs)
  - tool:write(app/src/**/*.rs)
---

# 宿主协程统一走进程级 runtime

## 规则

在 `src-tauri/src/` 与 `app/src/` 下，**禁止**直接调用 tokio 的协程创建函数：

```rust
// ❌ 错误：绕过进程级 runtime，任务入口分裂
tokio::spawn(async move { ... });
tokio::task::spawn(async move { ... });
tokio::spawn_blocking(|| { ... });
use tokio::task; task::spawn(async move { ... });
```

**必须**使用进程级 runtime 的统一入口：

```rust
// ✅ src-tauri
crate::runtime::spawn(async move { ... });
crate::runtime::spawn_blocking(|| { ... });

// ✅ app（GPUI 宿主经后端库重新导出）
zerolaunch_rs_lib::runtime::spawn(async move { ... });
zerolaunch_rs_lib::runtime::spawn_blocking(|| { ... });
```

需要从 GPUI 前台任务调用内核异步代码时，用 oneshot 把 future 投递到该 runtime，再回传结果，避免在 GPUI executor 上直接 poll 依赖 Tokio reactor 的 future。

## 原因

- 阶段 2 起 `src-tauri` 只是 Tauri 外壳，内核同时服务 CLI 与后续 GPUI 宿主；任务必须挂在同一个进程级 Tokio runtime 上。
- 直接 `tokio::spawn` 依赖当前线程是否处于 runtime 上下文，启动、测试与 CLI 路径容易在不同 runtime 上分裂。
- `tauri::async_runtime` 在启动时会被设置为同一 runtime 的 handle；内核代码不再直接依赖 Tauri 类型。

## 迁移期边界

- `src-tauri/src/lib.rs`、`bootstrap.rs`、`tray/` 等 Tauri 外壳代码允许调用 Tauri API，但创建任务仍走 `crate::runtime`。
- `src-tauri/src/runtime.rs` 是唯一 runtime 声明处；`app/` 经 `zerolaunch_rs_lib::runtime` 使用同一实例。
- 阶段 7 删除 `src-tauri` 时，`runtime` 模块迁入后端库 crate，本规则随目录删除。

## 范围边界

- **约束 `src-tauri/src/` 与 `app/src/`**：`crates/`（plugin-host、plugin-sdk-rust、plugin-protocol 等库）无 tauri 依赖，**必须**使用 tokio 原生 API，不在本规则范围。
- **`#[tokio::test]` 测试宏不在此列**：它是测试运行时宏而非协程创建函数，测试模块中可用。
- **`tokio::time::interval` / `tokio::time::sleep` 等挂起 API 不在此列**：它们不创建任务。
