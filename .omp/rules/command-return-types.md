---
description: IPC 命令返回类型约定 — 必须用命名结构体，禁止 serde_json::Value/json! 手动构造，列出已知例外
condition: "serde_json::json!|serde_json::Value|-> Result<"
scope: "tool:edit(src-tauri/src/commands/**), tool:write(src-tauri/src/commands/**)"
---

# IPC 命令返回类型约定

- 命令返回 `Result<T, BridgeError>`。`BridgeError` 在 `bridge_error.rs` 中定义
- `BridgeError` 包含 `code`（ErrorCode 枚举）和 `message`（人类可读描述）
- **正确**：`BridgeError::not_found(&component_id)` — 组件未找到
- **正确**：`BridgeError::internal(error_string)` — 内部错误
- 无数据返回的命令 **必须** 使用 `Result<(), BridgeError>`
- 返回类型 **必须** 是命名结构体（`#[derive(Serialize)]`）。**禁止** 直接返回 `serde_json::Value` 或 `serde_json::json!()` 手动构造的 JSON。
  - **正确**：`fn config_get_actions() -> Result<Vec<ConfigActionDef>, BridgeError>`
  - **错误**：`fn config_get_settings() -> Result<serde_json::Value, BridgeError>`
- 命名结构体 **必须** 在 Rust 中定义，前端在 `bridge/contract.ts` 中声明对应 TypeScript 类型。**禁止** 使用 `serde_json::json!({})` 局部构造返回值。
- 已知例外（返回值形状由动态 schema 或组件定义决定）：
  - `config_get_settings` — 返回值由组件的 schema 定义，形状不固定
  - `config_execute_action` — 每个组件可定义自己的 action 结果类型
  - `inspector_get_state` — 调试工具，输出随 feature flag 和插件状态变化
  - `cli_get_info` — CLI HTTP 服务器连接信息，形状由 token 文件结构决定
- 以上 exception 列表中，**凡返回 `Result<T, BridgeError>` 的命令**（如 `config_get_settings`、`config_execute_action`、`inspector_get_state` 等）**必须** 遵循 trace_id 追踪规范（生成 trace_id、`#[tracing::instrument]`、`.with_trace_id()`）。exception 仅豁免"禁止直接返回 `serde_json::Value`"这一项，**不豁免** trace_id 要求
