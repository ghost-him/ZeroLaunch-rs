---
description: IPC 命令错误处理 — 返回 Result<T, BridgeError>，用 ? 或 map_err 传播，禁止 unwrap/expect，内部错误通过 From 转换
condition: "BridgeError|map_err|\\.with_trace_id|\\.unwrap\\(|\\.expect\\("
scope: "tool:edit(src-tauri/src/commands/**), tool:write(src-tauri/src/commands/**)"
---

# IPC 命令错误处理

- 命令返回 `Result<T, BridgeError>`。**必须** 使用 `?` 或 `.map_err()` 传播错误
- **必须** 避免在命令处理器代码路径中使用 `unwrap()` 或 `expect()`
- `BridgeError` **仅** 在 commands 层（IPC 边界）使用。内部模块（SessionRouter、PluginManager 等）**必须** 定义自己的内部错误类型，通过 `From` 转换在 commands 层统一转为 `BridgeError`
- 命令处理器通过 `.with_trace_id(&trace_id)?` 将内部错误统一转为带 trace_id 的 BridgeError（`WithTraceId` trait 已为 `Result<T, E: Into<BridgeError>>` 实现，内部通过 `From` 转换，**禁止** 手写 `map_err(|e| BridgeError::from(e)...)`）
