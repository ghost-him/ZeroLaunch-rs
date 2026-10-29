---
description: IPC 命令错误处理 — 返回 Result<T, BridgeError>，用 ? 或 map_err 传播，禁止 unwrap/expect，内部错误通过 From 转换
condition: "BridgeError|map_err|\\.with_trace_id|\\.unwrap\\(|\\.expect\\("
scope: "tool:edit(src-tauri/src/commands/**), tool:write(src-tauri/src/commands/**)"
---

# IPC 命令错误处理

- 错误 **必须** 使用 `?` 或 `.map_err()` 传播，**禁止** 在命令处理器代码路径中使用 `unwrap()` 或 `expect()`
- `BridgeError` **仅** 在 commands 层（IPC 边界）使用。内部模块（SessionDispatcher、PluginManager 等）**必须** 定义自己的内部错误类型，通过 `From` 转换在 commands 层统一转为 `BridgeError`
- 内部错误转 BridgeError **必须** 走 `WithTraceId` 的 `.with_trace_id(&trace_id)?`，**禁止** 手写 `map_err(|e| BridgeError::from(e)...)`（trace_id 细节见 `trace-id` 规则）
