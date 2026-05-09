---
paths:
  - "src-tauri/src/plugin/**"
  - "src-tauri/src/plugin_system/**"
---

# 插件系统规范

## Configurable 生命周期铁律

`ConfigManager::apply_settings()` 按以下固定顺序执行，不可打乱、不可跳过、不可合并：

1. `validate_settings(&settings)?` — 纯校验，在此失败则阻断全部后续步骤
2. `apply_settings(settings)` — 仅写入内部 `RwLock`
3. `on_settings_changed()` — 副作用（重建服务、注册回调、启停监听器）
4. 通过广播通道发布 `ConfigEvent`
5. `save_to_storage()` — 持久化到本地 JSON，然后可选远程同步

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

- `ExecutorRegistry::execute()` 是动作执行的 **唯一** 公开入口。所有外部调用方通过 `execute()` 路由
- `ExecutorRegistry::resolve()` 和 `resolve_fallback()` 是 **私有的**。**禁止** 从 ExecutorRegistry 外部调用
- `ExecutorRegistry::get_actions()` 用于查询某 `TargetType` 的可用动作，仅用于查询，**禁止** 用于执行路由
- **正确**：`registry.execute(ctx, action_id).await`
- **错误**：`registry.resolve(target_type, action_id)` 从外部代码调用

## PluginHandle 使用

- 插件通过 `PluginHandle`（从 `HostApi::register()` 获取）访问平台能力。**禁止** 从 `plugin/` 代码直接 import 或调用 `sdk/platform/*`
- 如果某平台操作没有 `PluginHandle` 方法：先添加到 `PluginHandle`，再使用。**禁止** 绕过 `PluginHandle`
- 可用的 `PluginHandle` 方法：`get_icon()`、`shell_open()`、`shell_open_folder()`、`shell_execute_elevation()`、`shell_execute_command()`、`activate_window_by_process()`、`activate_window_by_title()`、`resolve_path()`、`enumerate_apps()`、`launch_app()`、`resolve_parameters()`、`count_user_parameters()`、`has_system_parameters()`

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

- `Plugin::init()` 接收 `Arc<dyn PluginAPI>`（横切：日志、通知、设置）和 `Arc<HostApi>`（平台服务）
- 用 `host_api` 参数执行平台操作。用 `api` 参数执行宿主级别的工具功能
- **禁止** 在插件内部状态中存储 `HostApi`；通过 `init` 参数或 `PluginContext` 访问
