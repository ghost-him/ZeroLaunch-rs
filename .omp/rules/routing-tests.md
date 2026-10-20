---
description: 确认/执行路由改动必须同步单测 — 动作改写、失败回退、开关门控分支要覆盖开/关两态
condition:
  - 'execute_candidate'
  - 'enter_wakes_first'
  - 'route_confirm'
  - 'ActivationFailed\s*\{'
scope:
  - tool:edit(src-tauri/src/plugin_framework/session_dispatcher.rs)
  - tool:write(src-tauri/src/plugin_framework/session_dispatcher.rs)
---

# 路由改动必须补单测

`execute_candidate` / `route_confirm` 的动作选择、动作改写与失败回退属于**纯行为差异**：编译通过、静态检查全绿，错也只是「多开一个实例 / 什么都不发生」。单测是唯一低成本的回归网。

## 约束

改动以下任一处时，**同一改动内** 在 `session_dispatcher.rs` 的 `mod tests` 补或更新用例：

- 动作改写（默认动作 → 唤醒动作）→ 覆盖开关开 / 关两态，断言实际执行的动作序列
- 失败回退（`ActivationFailed`）→ 覆盖 `launch_new_on_failure` 开 / 关两态
- 新增门控条件（配置项、目标类型、动作 ID）→ 每个条件各一条用例证明生效与不生效

## 写法

复用现有夹具：`RecordingExecutor`（记录执行动作序列）、`WindowBehaviorStub`（注入配置）、`seed_app_candidate`（写入候选缓存），直接调 `execute_candidate(candidate_id, 0, ACTION_EXECUTE, "", &[])` 并断言：

```rust
assert_eq!(*executed.lock(), vec![ACTION_ACTIVATE_WINDOW.to_string()]);
```

参照用例：`wake_existing_first_rewrites_default_action`、`wake_existing_first_disabled_keeps_default_action`、`wake_existing_first_does_not_rewrite_non_default_action`、`wake_failure_without_fallback_does_not_launch`、`wake_failure_with_fallback_launches_default_action`。

## 不要写

- 只断言 `is_ok()` / 不 panic（路由错误的形态恰恰是「成功且没做事」）
- 只测开关打开态（关态才是默认路径，回归最常发生在这里）
- 复制粘贴同一路径的重复用例
