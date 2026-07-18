---
description: 第三方插件 SDK Trace 模块 — span_for/with_trace/instrument 通过 PluginContext 注入 trace_id，禁止 with_trace 内 .await
condition: "trace::|span_for|with_trace|instrument|PluginContext"
scope: "tool:edit(crates/plugin-sdk-rust/**), tool:write(crates/plugin-sdk-rust/**), tool:edit(crates/plugin-api/src/plugin/**), tool:write(crates/plugin-api/src/plugin/**)"
---

# Trace 模块（第三方插件 SDK）

`crates/plugin-sdk-rust/src/trace.rs` 为第三方插件提供 tracing span 辅助，通过 `PluginContext` 自动注入 trace_id、plugin_id、query_id 等关联字段。

| 函数 | 签名 | 说明 |
|------|------|------|
| `span_for` | `fn span_for(ctx: &PluginContext) -> Span` | 根据 `PluginContext` 创建 `tracing::info_span!("plugin", trace_id, plugin_id, query_id)` |
| `with_trace` | `fn with_trace<R>(ctx: &PluginContext, f: impl FnOnce() -> R) -> R` | 在 span 内执行**同步**闭包。**禁止**在闭包内调用 async 运行时 |
| `instrument` | `fn instrument<F: Future>(ctx: &PluginContext, fut: F) -> Instrumented<F>` | 为异步 Future 附加 span，用法：`trace::instrument(&ctx, async { ... }).await` |

- trace_id 由宿主在调用 `Plugin::query()` / `Plugin::execute_action()` 时通过 `PluginContext` 传入
- 插件 **推荐** 在 hot-path 方法（`query`、`execute_action`）中使用 `trace::instrument` 包裹异步逻辑
- `with_trace` 仅用于纯同步代码，内部使用 `span.enter()` 持有守卫，**禁止** 在闭包内执行 `.await`
