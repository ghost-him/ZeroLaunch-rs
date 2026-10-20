---
description: 动作 ID 归声明它的执行器所有 — 内置执行器的动作 ID 常量留在宿主，禁止塞进 plugin-api 公共 SDK
condition:
  - '"activate_window"'
  - 'fallback_action\s*:\s*"'
  - 'action_id\s*(==|!=)\s*"'
  - 'action_id\s*:\s*"execute"|"actionId"\s*:\s*"execute"'
  - 'pub const ACTION_[A-Z_]+\s*:\s*&str'
scope:
  - tool:edit(src-tauri/src/builtin_plugin/executor/**)
  - tool:write(src-tauri/src/builtin_plugin/executor/**)
  - tool:edit(src-tauri/src/plugin_framework/**)
  - tool:write(src-tauri/src/plugin_framework/**)
  - tool:edit(crates/plugin-api/**)
  - tool:write(crates/plugin-api/**)
---

# 动作 ID 归属与常量位置

动作 ID（`ResultAction.id` / 确认载荷 `actionId` / `fallback_action`）按 `(TargetType, action_id)` 注册，**不是全局标识**：它属于声明它的那个执行器。常量跟着声明者走，不跟着「谁会用到」走。

## 分层

| 归属 | 常量位置 | 可见性 |
|---|---|---|
| 单个内置执行器的私有动作（如 `window-activate-executor` 的 `activate_window`） | 该执行器文件内 | `pub(crate)` |
| 多个内置执行器共用的默认动作（`execute`） | `src-tauri/src/builtin_plugin/executor/mod.rs` | `pub(crate)` |
| 第三方插件的动作 | 插件自己的代码内 | 插件私有 |
| SDK 默认动作（`ActionExecutor::supported_actions` 默认实现） | `crates/plugin-api/src/plugin/types.rs` 的内联值 | 不额外导出常量 |

- 跨模块消费方（`plugin_framework/session_dispatcher.rs`）从 `crate::builtin_plugin::executor::{ACTION_EXECUTE, ACTION_ACTIVATE_WINDOW}` 导入
- 内置执行器的动作常量经 `executor/mod.rs` 再导出，消费方**禁止** 深入私有子模块路径

## 禁止

- **禁止** 往 `crates/plugin-api` 添加内置执行器的动作 ID 常量（包括 `ACTION_ACTIVATE_WINDOW`）。`plugin-api` 是面向所有第三方开发者的公共契约面，内置实现的动作 ID 一旦进入 SDK，第三方会误以为它是公共契约、后续改不动
- **禁止** 在执行器或调度器里重复声明同值常量（`const WAKE_ACTION_ID` 这类）
- **禁止** 在构造/比较处写裸字面量：`fallback_action: "execute"`、`action_id == "execute"`、`action_id: "execute"`

```rust
// ❌ 内置动作进了公共 SDK
// crates/plugin-api/src/plugin/types.rs
pub const ACTION_ACTIVATE_WINDOW: &str = "activate_window";

// ❌ 重复声明
const WAKE_ACTION_ID: &str = "activate_window";

// ✅ 声明者持有、宿主消费
// builtin_plugin/executor/window_activate_executor.rs
pub(crate) const ACTION_ACTIVATE_WINDOW: &str = "activate_window";
// builtin_plugin/executor/mod.rs
pub(crate) const ACTION_EXECUTE: &str = "execute";
pub(crate) use window_activate_executor::ACTION_ACTIVATE_WINDOW;
```

## 原因

动作 ID 是字符串，不是类型：两处声明改一处另一处**没有编译错误**。调度器用 ID 判断「这是默认动作吗、这是唤醒动作吗」，声明处改名后路由退化成恒 false，表现是「开关打开了但回车不唤醒」，只能靠日志与真机复现。而把内置 ID 提升进 SDK，会让公共契约与宿主实现绑死，第三方插件作者无法判断哪些 ID 是自己可以用的。

## 豁免

- 插件**面板/行内动作**（`Plugin::execute_action` 的 action id，如 `builtin_plugin/detector/*` 的 `CustomPanel.actions`、translator 的 `copy_primary`）是插件私有的消息契约，不进入 `ExecutorRegistry` 路由，可用字面量；一旦它被宿主确认管道按 ID 裁决，就必须改走常量
- 只在单个文件内声明与使用的执行器私有动作（如 `PluginWakeExecutor` 的 `open`）用一个文件内 `const` 即可，不需要导出到 `executor/mod.rs`
