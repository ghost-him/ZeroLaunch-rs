---
description: trace_id 追踪规范 — CLI HTTP 请求与可失败宿主操作应带 trace_id 并建立 tracing span
condition: "trace_id|generate_trace_id|instrument"
scope: "tool:read(crates/zerolaunch-rs/src/cli_server/**), tool:edit(crates/zerolaunch-rs/src/cli_server/**), tool:write(crates/zerolaunch-rs/src/cli_server/**), tool:read(crates/zerolaunch-rs/src/core/**), tool:edit(crates/zerolaunch-rs/src/core/**), tool:write(crates/zerolaunch-rs/src/core/**), tool:read(crates/zerolaunch-rs/src/plugin_framework/**), tool:edit(crates/zerolaunch-rs/src/plugin_framework/**), tool:write(crates/zerolaunch-rs/src/plugin_framework/**), tool:read(crates/zerolaunch-rs/src/utils/**), tool:edit(crates/zerolaunch-rs/src/utils/**), tool:write(crates/zerolaunch-rs/src/utils/**)"
---

# trace_id 追踪规范

- CLI HTTP 入口由 `cli_server/middleware.rs` 生成或读取 `x-trace-id`，并建立 `http_request` span；路由处理函数直接使用 `Extension<TraceId>`，不重复生成。
- 会话查询、确认执行等可失败的核心操作用传入的 trace_id 关联日志。
- 新增 CLI 端点时复用中间件，不要另写请求 ID 逻辑。
- 插件协议请求的 trace_id 由 `PluginContext` 携带；宿主日志直接引用，不重新生成。
- trace_id 只用于日志关联，不进入用户可见错误文案。

```rust
let span = tracing::info_span!("http_request", trace_id = %trace_id);
```
