---
description: 内置插件注册时序 — Phase A inventory 注册全部 Configurable，Phase B 加载持久化配置后按 is_enabled 注册触发插件，再统一 init
condition: "Phase A|Phase B|load_from_storage|init_plugin_system|register_plugin|init_builtins|is_enabled|builtin.*init|启动时序|注册时序"
scope: "tool:read(src-tauri/src/bootstrap.rs), tool:edit(src-tauri/src/bootstrap.rs), tool:write(src-tauri/src/bootstrap.rs), tool:read(src-tauri/src/plugin_framework/session_dispatcher.rs), tool:edit(src-tauri/src/plugin_framework/session_dispatcher.rs), tool:write(src-tauri/src/plugin_framework/session_dispatcher.rs), tool:read(src-tauri/src/core/config/manager.rs), tool:edit(src-tauri/src/core/config/manager.rs), tool:write(src-tauri/src/core/config/manager.rs), tool:read(src-tauri/src/builtin_plugin/**), tool:edit(src-tauri/src/builtin_plugin/**), tool:write(src-tauri/src/builtin_plugin/**)"
---

# 内置插件注册时序（bootstrap）

`init_plugin_system`（`src-tauri/src/bootstrap.rs`）分阶段启动，顺序是**契约**，改动必须先理解依赖：

1. **Phase A — inventory 注册**：`PluginManager::init_builtins` → `builtin_registry::collect_all_builtin_entries()` 收集全部内置组件；所有 `Configurable` 注册进 `ConfigManager`（`register` 校验失败即拒绝）
2. **Phase B — 持久化配置**：`config_manager.load_from_storage()`；此后同步后端主题/语言
3. **按 is_enabled 注册触发插件**：`register_plugin(plugin, metadata, enabled)`，`enabled = config_manager.is_enabled(component_id)`——**必须放 Phase B 之后**，否则 `is_enabled` 读不到持久化结果，回退 `default_enabled`，导致用户禁用过的插件重启后仍启用
4. **统一 init 循环**：遍历 `plugin_registry().get_all()`，逐插件 `host_api.register(...)` 发句柄 + `plugin.init(&init_ctx, Some(handle))`。init_ctx.locale 携带**持久化语言**（Phase B 后可知），不能提前到 Phase A
5. **管道构建**：候选管道 + 偏置规则加载 + `rebuild_search_pipeline()`
6. **模型提供方**：`register_builtin_providers` + `refresh_models()`（依赖 Phase A 已注册的模型配置组件）
7. **第三方插件**：`load_all_third_party(...)`（依赖 `ConfigManager.loaded_config` 快照恢复保存配置）

## 约束

- **禁止** 把 Phase 3 的触发插件注册挪到 Phase A/load_from_storage 之前（is_enabled 失效）
- **禁止** 在 init 循环前提前发放 PluginHandle（内置插件 init 中保存句柄依赖循环完成）
- 改注册/初始化顺序时：检查每个阶段读取的状态是否已被上一阶段建立（is_enabled 需持久化、locale 需持久化语言、模型配置组件需 Phase A 注册）
