---
paths:
  - "src-tauri/src/commands/**"
  - "src-ui/bridge/**"
  - "src-tauri/src/cli_server/**"
---

# Tauri Command 规范

## 命名与文件放置

- 所有命令 **必须** 在 `lib.rs` 中通过 `generate_handler![]` 注册
- 前缀与文件对应关系（具体命令以各文件中 `#[tauri::command]` 标注为准）：

| 前缀 | 文件 | 域 |
|------|------|-----|
| `bridge_` | `commands/bridge.rs` | 搜索/会话 |
| `config_` | `commands/config_file.rs` | 配置管理 |
| `resource_` | `commands/resource.rs` | 资源管理 |
| `plugin_` | `commands/plugin.rs` | 插件管理 |
| `inspector_` | `commands/inspector.rs` | 插件检查器 |
| `cli_` | `commands/cli.rs` | CLI HTTP 服务器 |

- 引入新前缀时 **必须** 同步更新此表

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


## 错误处理

- 命令返回 `Result<T, BridgeError>`。**必须** 使用 `?` 或 `.map_err()` 传播错误
- **必须** 避免在命令处理器代码路径中使用 `unwrap()` 或 `expect()`
- `BridgeError` **仅** 在 commands 层（IPC 边界）使用。内部模块（SessionRouter、PluginManager 等）**必须** 定义自己的内部错误类型，通过 `From` 转换在 commands 层统一转为 `BridgeError`
- 命令处理器通过 `.with_trace_id(&trace_id)?` 将内部错误统一转为带 trace_id 的 BridgeError（`WithTraceId` trait 已为 `Result<T, E: Into<BridgeError>>` 实现，内部通过 `From` 转换，**禁止** 手写 `map_err(|e| BridgeError::from(e)...)` ）

## trace_id 追踪

- 所有返回 `Result<T, BridgeError>` 的命令 **必须** 在函数体开头生成 trace_id 并建立 tracing span：
  ```rust
  #[tracing::instrument(skip(state), fields(trace_id))]
  pub async fn xxx(state: ...) -> Result<T, BridgeError> {
      let trace_id = crate::utils::trace_id::generate_trace_id();
      tracing::Span::current().record("trace_id", trace_id.as_str());
      // ...
  }
  ```
- `fields(trace_id)` **必须** 保留（声明元数据字段）—— 否则 `record()` 无法将值写入元数据，`tracing-subscriber` 格式化时不会显示 `trace_id`，导致并发日志无法按 trace_id 追踪。
- `record` **必须** 使用 `trace_id.as_str()`，**禁止** 使用 `tracing::field::display(&trace_id)`。原因：`field::display()` 不更新元数据字段，而是把值存入 span 扩展，与 `fields(trace_id)` 声明的空字段并存，导致同一字段格式化时出现两次（`trace_id=xxx trace_id=xxx`）。`as_str()` 则将值正确写入元数据字段，只出现一次。
- **特例**：当 trace_id 来自**函数参数**而非内部生成时，可使用 `fields(trace_id = %trace_id)` 直接初始化（如 `route_query`、`route_confirm`）。此时无需 `record()` 调用。
- **必须** 使用 `crate::utils::trace_id::generate_trace_id()` 统一生成，**禁止** 各处自行用 `Uuid::new_v4()` 或 `rand` 生成
- `BridgeError` **必须** 包含非空 `trace_id`。所有错误路径 **必须** 调用 `.with_trace_id(&trace_id)`
- 成功响应 **不携带** trace_id（成功时用户不需要排查）
- async 函数 **必须** 用 `#[tracing::instrument]`，**禁止** `span.enter()` 跨 `.await`
- trace_id **必须** 排除在命令签名之外（不在参数中暴露，由命令内部生成）
- CLI HTTP 服务器 **必须** 通过 `X-Trace-Id` 请求头/响应头传递 trace_id
- **trace_id 适用范围**：所有返回 `Result<T, BridgeError>` 的命令 **必须** 生成 trace_id（无论是否真的有错误路径；trace_id 同时用于 span 日志关联，如查看操作耗时与事件顺序），**禁止** 因"无错误路径"而省略模板代码。不返回 `Result<T, BridgeError>` 的命令（如返回 `String`、`usize`、`Vec<T>`、`()` 等）不需要 trace_id。需 trace_id 的示例：`bridge_query`、`bridge_refresh_candidates`、`bridge_hide_window`。无需 trace_id 的示例：`config_get_version() -> String`、`config_get_actions() -> Vec<ConfigActionDef>`、`bridge_get_session_mode() -> String`
- 核心热路径中的 async 方法（如 `SessionRouter::route_query`、`route_confirm`、`execute_candidate` 等）**推荐** 使用 `#[tracing::instrument]`，以便在 spans 中捕获 trace_id 实现端到端追踪。trace_id 来自参数时用 `fields(trace_id = %trace_id)`；不需要 trace_id 的方法（如 `on_search_bar_wake`）用 `#[tracing::instrument(skip(self))]` 即可，trace_id 由父 span 继承。

## 返回类型约定

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
  - `inspector_get_state` / `inspector_simulate_query` — 调试工具，输出随 feature flag 和插件状态变化
  - `cli_get_info` — CLI HTTP 服务器连接信息，形状由 token 文件结构决定
- 以上 exception 列表中，**凡返回 `Result<T, BridgeError>` 的命令**（如 `config_get_settings`、`config_execute_action`、`inspector_get_state` 等）**必须** 遵循 trace_id 追踪规范（生成 trace_id、`#[tracing::instrument]`、`.with_trace_id()`）。exception 仅豁免"禁止直接返回 `serde_json::Value`"这一项，**不豁免** trace_id 要求
