---
paths:
  - "src-tauri/src/commands/**"
  - "src-ui/bridge/**"
  - "src-tauri/src/cli_server/**"
---

# Tauri Command 规范

## 命名前缀

- 搜索/会话命令：**必须** 用 `bridge_` 前缀（具体命令以 `commands/bridge.rs` 中 `#[tauri::command]` 标注为准）
- 配置命令：**必须** 用 `config_` 前缀（具体命令以 `commands/config_file.rs` 中 `#[tauri::command]` 标注为准）
- 资源命令：**必须** 用 `resource_` 前缀（具体命令以 `commands/resource.rs` 中 `#[tauri::command]` 标注为准）
- 插件管理命令：**必须** 用 `plugin_` 前缀（具体命令以 `commands/plugin.rs` 中 `#[tauri::command]` 标注为准）
- 检查器命令：**必须** 用 `inspector_` 前缀（具体命令以 `commands/inspector.rs` 中 `#[tauri::command]` 标注为准）
- CLI 命令：**必须** 用 `cli_` 前缀（具体命令以 `commands/cli.rs` 中 `#[tauri::command]` 标注为准）
- **必须** 使用正确前缀。引入新前缀时 **必须** 同步更新此规则文件

## 参数约定

- 0～2 个简单参数的命令：使用扁平参数
  - **正确**：`fn bridge_query(raw_query: String)`
- 3 个及以上参数的命令：使用单个反序列化结构体
  - **正确**：`fn bridge_confirm(payload: ConfirmPayload)`
- 所有结构体参数 **必须** `#[derive(Deserialize)]`

## 序列化契约

- IPC 类型 JSON 键名统一使用 **camelCase**，Rust 与 TypeScript 两侧 **必须** 一致。详细标注规则见 Serde 序列化规范
- 前端类型在 `bridge/contract.ts` 中 **必须** 与 Rust 类型 `commands/bridge.rs` 和 `commands/config_file.rs` 保持同步
- 新增/重命名 Rust IPC 类型字段时：**同一 commit** 中更新 `bridge/contract.ts`

## Serde 序列化规范

- **必须** 使用字段级 `#[serde(rename = "xxx")]` 显式标注每个字段和 variant 的 JSON 键名。即使 Rust 字段名与 JSON 键名相同，也必须显式标注以保持风格统一和可读性。`rename_all` 在 externally tagged enum 上只会重命名 variant 标签名，**不会** 重命名 variant 内部的字段名，导致前后端字段名不一致。
- **正确**：
  ```rust
  #[derive(Serialize, Deserialize)]
  pub enum MyEnum {
      // 1. 显式标注变体标签名
      #[serde(rename = "myVariant")]
      MyVariant {
          // 2. 显式标注变体内部的每一个字段
          #[serde(rename = "innerField")]
          inner_field: String,
          #[serde(rename = "count")]
          count: u32,
      },
      
      // 无内部字段的变体也需标注
      #[serde(rename = "unitVariant")]
      UnitVariant, 
  }
  ```
- enum unit variant 和 variant with fields 均需显式标注：variant 标签加 `#[serde(rename = "...")]`，有内部字段的 variant 还需对每个内部字段标注。


## 命令注册

- 所有命令 **必须** 在 `lib.rs` 中通过 `generate_handler![]` 注册
- Bridge 命令 **必须** 放在 `commands/bridge.rs` 中
- Config 命令 **必须** 放在 `commands/config_file.rs` 中
- 资源命令 **必须** 放在 `commands/resource.rs` 中
- 插件管理命令 **必须** 放在 `commands/plugin.rs` 中
- 检查器命令 **必须** 放在 `commands/inspector.rs` 中
- CLI 命令 **必须** 放在 `commands/cli.rs` 中

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
