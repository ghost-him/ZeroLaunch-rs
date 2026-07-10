---
description: 禁止在 IPC 命令处理器中使用 unwrap/expect — 会导致进程 panic
condition: "\\.(unwrap|expect)\\("
scope: "tool:edit(src-tauri/src/commands/**), tool:write(src-tauri/src/commands/**)"
---

你在 commands 层代码中写了 `unwrap()` 或 `expect()`。IPC 命令处理器必须用 `?` 或 `.map_err()` 传播错误，返回 `Result<T, BridgeError>`。

`unwrap()`/`expect()` 在用户输入异常或平台 API 失败时会导致整个进程 panic，用户无法得到错误信息。

改用错误传播：
- `?` 运算符：`state.config.get(&id)?`
- `.map_err()`：`result.map_err(|e| BridgeError::internal(e.to_string()))`
- `.with_trace_id(&trace_id)?` 用于将内部错误转为带 trace_id 的 BridgeError
