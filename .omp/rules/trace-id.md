---
description: trace_id 追踪规范 — 所有返回 Result<T, BridgeError> 的命令必须生成 trace_id 并建立 tracing span，核心热路径推荐使用
condition: ".*"
scope: "tool:read(src-tauri/src/commands/**), tool:edit(src-tauri/src/commands/**), tool:write(src-tauri/src/commands/**), tool:read(src-tauri/src/cli_server/**), tool:edit(src-tauri/src/cli_server/**), tool:write(src-tauri/src/cli_server/**), tool:read(src-tauri/src/core/**), tool:edit(src-tauri/src/core/**), tool:write(src-tauri/src/core/**)"
---

# trace_id 追踪规范

## 命令层（必须）

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

## 核心热路径（推荐）

核心热路径中的 async 方法（如 `SessionRouter::route_query`、`route_confirm`、`execute_candidate` 等）**推荐** 使用 `#[tracing::instrument]`，以便在 spans 中捕获 trace_id 实现端到端追踪。trace_id 来自参数时用 `fields(trace_id = %trace_id)`；不需要 trace_id 的方法（如 `on_search_bar_wake`）用 `#[tracing::instrument(skip(self))]` 即可，trace_id 由父 span 继承。
