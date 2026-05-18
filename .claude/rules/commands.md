---
paths:
  - "src-tauri/src/commands/**"
  - "src-ui-new/bridge/**"
---

# Tauri Command 规范

## 命名前缀

- 搜索/会话命令：**必须** 用 `bridge_` 前缀
  - `bridge_query`、`bridge_confirm`、`bridge_wake`、`bridge_reset`、`bridge_get_session_mode`、`bridge_refresh_candidates`、`bridge_get_candidates_count`
- 配置命令：**必须** 用 `config_` 前缀
  - `config_get_all_components`、`config_get_schema`、`config_get_settings`、`config_apply_settings`、`config_reset_settings`、`config_set_enabled`、`config_get_actions`、`config_execute_action`
- 资源命令：**必须** 用 `resource_` 前缀
  - `resource_get`、`resource_upload`
- **禁止** 混用前缀。**禁止** 不更新此规则文件就引入新前缀

## 参数约定

- 0～2 个简单参数的命令：使用扁平参数
  - **正确**：`fn bridge_query(raw_query: String)`
- 3 个及以上参数的命令：使用单个反序列化结构体
  - **正确**：`fn bridge_confirm(payload: ConfirmPayload)`
- 所有结构体参数 **必须** `#[derive(Deserialize)]` 并使用 `#[serde(rename_all = "camelCase")]`

## 序列化契约

- 所有 IPC 类型（双向）使用 **camelCase** 字段名
- Rust 侧：跨 IPC 边界的 **每个** 结构体/枚举都要加 `#[serde(rename_all = "camelCase")]`
- TypeScript 侧：interface/type 字段使用与 Rust 定义匹配的 camelCase
- 前端类型在 `bridge/contract.ts` 中 **必须** 与 Rust 类型 `commands/bridge.rs` 和 `commands/config_file.rs` 保持同步
- 新增/重命名 Rust IPC 类型字段时：**同一 commit** 中更新 `bridge/contract.ts`

## 命令注册

- 所有命令 **必须** 在 `lib.rs` 中通过 `generate_handler![]` 注册
- Bridge 命令在 `commands/bridge.rs` 中。**禁止** 在其他地方添加 bridge 命令
- Config 命令在 `commands/config_file.rs` 中。**禁止** 在其他地方添加 config 命令
- 资源命令在 `commands/resource.rs` 中。**禁止** 在其他地方添加资源命令

## 错误处理

- 命令返回 `Result<T, String>` 或 `Result<T, BridgeError>`。**禁止** 在命令处理器中 panic
- **禁止** 在命令处理器代码路径中使用 `unwrap()` 或 `expect()`。使用 `?` 或 `.map_err()`

## trace_id

- `trace_id` 由后端（`lib.rs`）生成。**禁止** 从前端接收 `trace_id`。**禁止** 将其添加到命令签名中

## 返回类型约定

- 命令返回 `Result<T, BridgeError>`。`BridgeError` 在 `core/types/bridge_error.rs` 中定义
- `BridgeError` 包含 `code`（ErrorCode 枚举）和 `message`（人类可读描述）
- **正确**：`BridgeError::not_found(&component_id)` — 组件未找到
- **正确**：`BridgeError::internal(error_string)` — 内部错误
- **禁止** 返回裸 `String` 错误。**必须** 使用 `BridgeError` 包装
- 无数据返回的命令使用 `Result<(), BridgeError>`
