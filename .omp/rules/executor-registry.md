---
description: ExecutorRegistry 使用 — resolve/resolve_fallback 是动作执行器唯一查找入口，get_actions 仅查询不执行
condition: "ExecutorRegistry|resolve\\(|resolve_fallback|get_actions"
scope: "tool:read(src-tauri/src/plugin_framework/executor_registry.rs), tool:read(src-tauri/src/plugin_framework/session_dispatcher.rs), tool:edit(src-tauri/src/**), tool:write(src-tauri/src/**), tool:edit(crates/plugin-api/src/**), tool:write(crates/plugin-api/src/**)"
---

# ExecutorRegistry

- `ExecutorRegistry::resolve(ctx, action_id)` 是动作执行器的 **唯一** 查找入口，返回 `Arc<dyn ActionExecutor>`
- `ExecutorRegistry::resolve_fallback(ctx, fallback_action)` 用于窗口唤醒失败时的回退执行器查找
- `ExecutorRegistry::get_actions(target_type)` 用于查询某 `TargetType` 的可用动作，仅用于查询，**禁止** 用于执行路由
- 调用方从 `resolve()` / `resolve_fallback()` 获取 executor 后，再调用 `executor.execute(ctx, action_id).await`
- 参照实现：`plugin_framework/session_dispatcher.rs` 的 `execute_candidate`（先
  `ExecutorRegistry::resolve` 再 `executor.execute`；`ActivationFailed { fallback_action }`
  时按配置走 `resolve_fallback` 回退执行）

## 动作改写 / 回退策略同样是执行路由

判断「该目标类型有没有唤醒动作」「这是不是默认动作」属于**路由裁决**，不是查询展示：

```rust
// ❌ 用查询接口做路由：动作列表是展示用投影，不保证与 execute 的可执行性一致
let actions = registry.get_actions(ctx.target.target_type());
actions.iter().any(|a| a.id == ACTION_ACTIVATE_WINDOW)

// ✅ 用查找接口做能力探针：resolve 失败即该动作不可执行
registry.resolve(ctx, ACTION_ACTIVATE_WINDOW).is_ok()
```

`get_actions` 只用于构造前端动作列表（`ListItem.actions`）与 CLI/检查器展示。
