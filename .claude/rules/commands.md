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
- **必须** 使用正确前缀。引入新前缀时 **必须** 同步更新此规则文件

## 参数约定

- 0～2 个简单参数的命令：使用扁平参数
  - **正确**：`fn bridge_query(raw_query: String)`
- 3 个及以上参数的命令：使用单个反序列化结构体
  - **正确**：`fn bridge_confirm(payload: ConfirmPayload)`
- 所有结构体参数 **必须** `#[derive(Deserialize)]`。每个字段 **必须** 显式标注 `#[serde(rename = "camelCaseKey")]`（见 [general.md](general.md) 的 Serde 序列化规范）

## 序列化契约

- 所有 IPC 类型（双向）的 JSON 键名使用 **camelCase**
- Rust 侧：跨 IPC 边界的每个结构体/枚举字段 **必须** 显式标注 `#[serde(rename = "camelCaseKey")]`。详细规则见 [general.md](general.md) 的 Serde 序列化规范
- TypeScript 侧：interface/type 字段使用与 Rust `#[serde(rename = "...")]` 匹配的 camelCase
- 前端类型在 `bridge/contract.ts` 中 **必须** 与 Rust 类型 `commands/bridge.rs` 和 `commands/config_file.rs` 保持同步
- 新增/重命名 Rust IPC 类型字段时：**同一 commit** 中更新 `bridge/contract.ts`

## 命令注册

- 所有命令 **必须** 在 `lib.rs` 中通过 `generate_handler![]` 注册
- Bridge 命令 **必须** 放在 `commands/bridge.rs` 中
- Config 命令 **必须** 放在 `commands/config_file.rs` 中
- 资源命令 **必须** 放在 `commands/resource.rs` 中

## 错误处理

- 命令返回 `Result<T, BridgeError>`。**必须** 使用 `?` 或 `.map_err()` 传播错误
- **必须** 避免在命令处理器代码路径中使用 `unwrap()` 或 `expect()`

## trace_id

- `trace_id` **必须** 由后端（`lib.rs`）生成，**必须** 排除在命令签名之外

## 返回类型约定

- 命令返回 `Result<T, BridgeError>`。`BridgeError` 在 `core/types/bridge_error.rs` 中定义
- `BridgeError` 包含 `code`（ErrorCode 枚举）和 `message`（人类可读描述）
- **正确**：`BridgeError::not_found(&component_id)` — 组件未找到
- **正确**：`BridgeError::internal(error_string)` — 内部错误
- **必须** 使用 `BridgeError` 包装错误，**必须** 避免返回裸 `String`
- 无数据返回的命令 **必须** 使用 `Result<(), BridgeError>`
