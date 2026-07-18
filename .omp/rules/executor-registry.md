---
description: ExecutorRegistry 使用 — resolve/resolve_fallback 是动作执行器唯一查找入口，get_actions 仅查询不执行
condition: "ExecutorRegistry|resolve\\(|resolve_fallback|get_actions"
scope: "tool:edit(src-tauri/src/**), tool:write(src-tauri/src/**), tool:edit(crates/plugin-api/src/**), tool:write(crates/plugin-api/src/**)"
---

# ExecutorRegistry

- `ExecutorRegistry::resolve(ctx, action_id)` 是动作执行器的 **唯一** 查找入口，返回 `Arc<dyn ActionExecutor>`
- `ExecutorRegistry::resolve_fallback(ctx, fallback_action)` 用于窗口唤醒失败时的回退执行器查找
- `ExecutorRegistry::get_actions(target_type)` 用于查询某 `TargetType` 的可用动作，仅用于查询，**禁止** 用于执行路由
- 调用方从 `resolve()` / `resolve_fallback()` 获取 executor 后，再调用 `executor.execute(ctx, action_id).await`
- 参照实现：`session_router.rs` 的 `route_confirm()` — 先 resolve 再 execute，含 fallback 处理
