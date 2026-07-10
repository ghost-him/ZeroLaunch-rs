---
description: 禁止在 IPC 命令返回值中使用 serde_json::json! 手动构造 — 必须用命名结构体
condition: "serde_json::json!"
scope: "tool:edit(src-tauri/src/commands/**), tool:write(src-tauri/src/commands/**)"
---

你在 IPC 命令中使用了 `serde_json::json!()` 手动构造返回值。命令返回类型必须是命名结构体（`#[derive(Serialize)]`），在 Rust 中定义，前端在 `bridge/contract.ts` 中声明对应 TypeScript 类型。

正确：`fn config_get_actions() -> Result<Vec<ConfigActionDef>, BridgeError>`
错误：`fn config_get_settings() -> Result<serde_json::Value, BridgeError>`

已知例外（返回值形状由动态 schema 决定，允许返回 `serde_json::Value`）：
- `config_get_settings` — 返回值由组件 schema 定义
- `config_execute_action` — 每个组件可定义自己的 action 结果类型
- `inspector_get_state` / `inspector_simulate_query` — 调试工具
- `cli_get_info` — CLI 连接信息
