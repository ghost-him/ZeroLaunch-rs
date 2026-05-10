---
paths:
  - "src-tauri/src/plugin/**"
  - "src-tauri/src/plugin_system/**"
---

# 插件系统规范

## Configurable 生命周期铁律

配置变更的 5 步流水线顺序见 [config.md](config.md) 的 `ConfigManager 的 save/load 流水线`。此处从组件实现者视角补充各方法的职责边界：

### validate_settings

- **只做**：纯校验 — 检查枚举范围、格式正确性、必要字段
- **禁止**：修改状态、执行 I/O、调用 `HostApi`、注册回调

### apply_settings

- **只做**：将配置写入组件的内部 `RwLock`。不做任何其他副作用
- **禁止**：重建外部服务、启动/停止监听器、注册热键、调用 `HostApi`

### on_settings_changed

- **只做**：执行副作用 — 重建服务、注册/取消注册回调、启动/停止监听器
- **禁止**：修改配置值（此时配置已生效）

参照实现：`HotkeyConfigComponent`、`InstallationMonitorConfigComponent`。

### 反模式

- **禁止** 把校验逻辑放在 `apply_settings` 中。放入 `validate_settings`
- **禁止** 把副作用放在 `apply_settings` 中。放入 `on_settings_changed`

## ExecutorRegistry

- `ExecutorRegistry::resolve(ctx, action_id)` 是动作执行器的 **唯一** 查找入口，返回 `Arc<dyn ActionExecutor>`
- `ExecutorRegistry::resolve_fallback(ctx, fallback_action)` 用于窗口唤醒失败时的回退执行器查找
- `ExecutorRegistry::get_actions(target_type)` 用于查询某 `TargetType` 的可用动作，仅用于查询，**禁止** 用于执行路由
- 调用方从 `resolve()` / `resolve_fallback()` 获取 executor 后，再调用 `executor.execute(ctx, action_id).await`
- 参照实现：`session_router.rs` 的 `route_confirm()` — 先 resolve 再 execute，含 fallback 处理

## PluginHandle 使用

- 插件通过 `PluginHandle`（从 `HostApi::register()` 获取）访问平台能力。**禁止** 从 `plugin/` 代码直接 import 或调用 `sdk/platform/*`
- 如果某平台操作没有 `PluginHandle` 方法：先添加到 `PluginHandle`，再使用。**禁止** 绕过 `PluginHandle`
- 可用的方法列表请直接阅读 `PluginHandle` 源码（`src-tauri/src/sdk/host_api.rs`），以代码为准，避免文档滞后

## Inner 模式（RwLock\<Inner\> 组件）

- 外壳方法仅做 **一行** 委托：`self.inner.read().method()` 或 `self.inner.write().method()`
- 外壳方法签名 **必须** 与 Inner 方法签名完全一致
- 所有业务逻辑在 Inner 的 `impl` 块中。外壳 `impl` 块仅包含锁委托
- **禁止** 把业务逻辑放入外壳的 `impl` 块

## 依赖方向

- `sdk/` → **禁止** 引用 `core/`、`plugin/`、`plugin_system/`
- `core/` → **禁止** 引用 `plugin/`、`plugin_system/`
- `plugin/` → 可引用 `core/` 和 `sdk/`
- `plugin_system/` → 可引用 `plugin/`、`core/`、`sdk/`
- **禁止** 添加反向依赖引用

## Plugin Trait Init

- `Plugin::init()` 接收 `&PluginContext`（请求级上下文）和 `Arc<HostApi>`（平台服务）
- 用 `host_api` 参数执行平台操作。用 `ctx` 参数获取 trace_id、query_id 等请求级信息
- **禁止** 在插件内部状态中存储 `HostApi`；通过 `init` 参数或 `PluginContext` 访问
