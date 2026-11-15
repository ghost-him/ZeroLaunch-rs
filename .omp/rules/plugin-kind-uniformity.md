---
name: plugin-kind-uniformity
description: 内置插件与第三方插件只差存放位置/执行方式，生命周期与能力契约必须统一；禁止按 PluginKind 分支改变行为
condition:
  - 'PluginKind'
  - 'on_shutdown'
  - 'plugin_methods::SHUTDOWN'
  - 'components\.provides'
  - 'ComponentKind'
  - 'provides'
  - '锚点'
  - 'normalize_panel_type'
  - 'enabled_plugins'
  - 'canUninstall'
  - 'canViewLogs'
scope:
  - tool:edit(crates/plugin-api/src/**)
  - tool:write(crates/plugin-api/src/**)
  - tool:edit(crates/plugin-protocol/src/**)
  - tool:write(crates/plugin-protocol/src/**)
  - tool:edit(crates/plugin-sdk-rust/src/**)
  - tool:write(crates/plugin-sdk-rust/src/**)
  - tool:edit(crates/plugin-host/src/**)
  - tool:write(crates/plugin-host/src/**)
  - tool:edit(crates/plugin-host/tests/**)
  - tool:write(crates/plugin-host/tests/**)
  - tool:edit(src-tauri/src/plugin_framework/**)
  - tool:write(src-tauri/src/plugin_framework/**)
  - tool:edit(src-tauri/src/builtin_plugin/**)
  - tool:write(src-tauri/src/builtin_plugin/**)
  - tool:edit(src-tauri/src/bootstrap.rs)
  - tool:write(src-tauri/src/bootstrap.rs)
  - tool:edit(src-tauri/src/commands/**)
  - tool:write(src-tauri/src/commands/**)
  - tool:edit(src-tauri/src/lib.rs)
  - tool:write(src-tauri/src/lib.rs)
  - tool:edit(src-ui/views/settings/**)
  - tool:write(src-ui/views/settings/**)
  - tool:edit(src-ui/composables/**)
  - tool:write(src-ui/composables/**)
  - tool:edit(src-ui/bridge/**)
  - tool:write(src-ui/bridge/**)
---

# 插件类型只有存放位置不同

`PluginKind::{Builtin, ThirdParty}` **只**描述实现位置与执行方式：

| 维度 | 内置 | 第三方 |
|---|---|---|
| 代码存放 | 编译进宿主二进制 | 独立安装目录 + 子进程 |
| 调用方式 | 进程内 trait 直调 | stdio JSON-RPC |
| 平台能力 | `init` 注入 `PluginHandle` | SDK `host()` 反向 RPC |

**除此之外，插件级契约必须完全一致**：`Plugin` / `Configurable` / `DataSource` / `ActionExecutor`
等 trait 的语义、生命周期时机、错误处理、默认实现、禁用/启用规则都不得因 `kind` 而不同。

## 禁止：调用点按 kind 分支改变语义

❌ 错误 —— 同一生命周期钩子只对内置执行：

```rust
for (plugin, metadata) in registry.get_all_with_metadata() {
    if metadata.kind != PluginKind::Builtin {
        continue;
    }
    plugin.on_shutdown(&ctx).await?;
}
```

✅ 正确 —— 调用点只认 trait，差异全部由适配器内部吸收：

```rust
for (plugin, metadata) in registry.get_all_with_metadata() {
    plugin.on_shutdown(&ctx).await?;
}
```

`kind` 只允许用于**展示/诊断**（如前端区分"内置/第三方"标签）和**必须的进程边界判断**
（如禁止对进程内插件发送 RPC）；不得用于决定某个契约是否生效。

## 新增生命周期能力：三处同步

任何插件级生命周期方法（如 `init` / `on_shutdown`）必须一次改齐：

1. **`Plugin` trait**（`crates/plugin-api/src/plugin/plugin_trait.rs`）：默认实现保证源码兼容；
2. **远端适配器**（`crates/plugin-host/src/adapter/remote_component.rs`）：覆写并把调用翻译为
   对应的 `plugin/*` RPC（含 `METHOD_NOT_FOUND` 容错，旧插件视为无该逻辑）；
3. **调用点**：内置与第三方共用同一条路径（应用退出遍历注册表、`teardown` 卸载），
   禁止在调用点写 `if kind == ...`。

## 禁止双发：进程层不再重复发送同一语义

RPC 由适配器发一次即可。进程关闭基础设施（`PluginProcess::shutdown`、
`PluginHostManager::shutdown_loaded_process` / `shutdown_all`）**不得**再私下发送
`plugin/shutdown` —— 否则应用退出时会执行两次钩子。

- ❌ `PluginProcess::shutdown` 内直接 `client.call(plugin_methods::SHUTDOWN, ...)`
- ✅ `RemoteComponent::on_shutdown` 发 RPC；`PluginProcess::shutdown` 只做
  "标记 Stopped → 关 stdin → 等待退出 → 超时强杀"

## 进程边界允许的差异（必须封装在适配器内）

- 超时：远端 RPC 有上限（`SHUTDOWN_HOOK_TIMEOUT`），进程内直调无超时；
- 错误传播：远端把 `ProtocolError` 转 `PluginError`，进程内直接返回 `PluginError`；
- 上下文来源：远端 `PluginContext` 由调用方构造后随 RPC 下发。

这些差异**不得**泄漏成调用点的 `kind` 分支，也不得改变"每个插件恰好执行一次"的语义。

## 派生状态：单一计算点，调用点不接触 kind

插件级展示/行为状态一律由宿主派生一次，前端与命令层只消费结果：

| 状态 | 唯一计算点 | 消费方 |
|---|---|---|
| 能力位（`can_uninstall` / `can_reload` / `can_view_logs` / `can_toggle`） | `PluginInfo::from_metadata`（kind + state） | 前端按钮显隐/开关禁用 |
| 插件级启用态 `enabled` | `PluginInfo::from_metadata`，取 Plugin 组件（id == plugin_id）的启用态 | 列表/详情/安装返回值 |
| 面板 provider key | `SessionDispatcher::normalize_panel_type`（内部查注册表） | UI 查询、热键唤醒、CLI 查询 |
| 组件类型 `ComponentType` | 宿主从 `ComponentKind` 派生（协议不再上报） | ConfigManager 事件路由/管道注册 |

- 前端**不得**再用 `kind === 'builtin' / 'third-party'` 决定动作可用性；`kind` 只用于展示文案。
- 安装/重载等命令的返回值必须复用同一视图构造函数（如 `PluginManager::plugin_info`），不得另算 `enabled`。
- 同一事实的缺省不得在不同调用点不同：面板 key 的 kind 解析只有一处；元数据缺失时保留原值并告警，不猜类型。

## 锚点：Plugin 组件 id 必须等于 plugin_id

- 一个插件包有且只有一个 `ComponentKind::Plugin` 组件；其 `ComponentCore::component_id` **必须**等于清单 `[plugin].id`。
- 其他组件（DataSource / ActionExecutor / SearchEngine / ...）各自使用独立 id，不受此约束。
- 有了该不变量：
  - `EnabledChanged` 的 Plugin 分支直接用 `component_id` 作为插件启用键，无需映射表；
  - 前端设置页按 `plugin_id` 匹配组件配置，无需额外下发组件 id；
  - `PluginInfo` 不再携带 `componentIds` 全量列表（管道配置由 `config_get_all_components` 提供）。

## 组件集合契约：加载期交叉校验

第三方组件的身份由插件自报，宿主必须在 `load` 阶段（登记前）校验，不通过即关进程拒绝加载：

1. **恰好一个** Plugin 组件（生命周期钩子与触发词路由都挂在它上面），且其组件 id **必须等于** `[plugin].id`；
2. `components.provides` 与实际 `ComponentKind` 集合**完全一致**；
3. 协议只上报 `kind`；`ComponentType` 由宿主派生，禁止插件重复声明；
4. 组件 id 冲突预检集合必须包含**全部**内置组件 id，包括非 inventory 手工注册的（如 `plugin-wake-executor`）。

## 自查

- 这个 `if metadata.kind` / `match kind` 是在改变契约行为吗？→ 删掉，下沉到适配器或改为能力位。
- 前端是否又在用 `kind` 决定按钮/开关？→ 改用 `PluginInfo` 能力位。
- 新增的 trait 生命周期方法，`RemoteComponent` 覆写了吗？
- 是否同时在进程关闭路径里又发了一次同名 RPC？→ 删除其一，保持单一权威。
- 新增组件种类时，`ComponentKind`、宿主 `component_type_of`、`provides` 映射三处是否同步？
- 新增启用事件入口时，Plugin 组件的 `component_id` 是否等于 `plugin_id`？
