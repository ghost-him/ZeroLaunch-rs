---
description: Plugin Trait Init — init 接收 PluginContext 和 Arc<PluginHandle>，禁止在插件内部状态存储 PluginHandle
condition: "Plugin::init|PluginContext|fn init"
scope: "tool:edit(src-tauri/src/**), tool:write(src-tauri/src/**), tool:edit(crates/plugin-api/src/plugin/**), tool:write(crates/plugin-api/src/plugin/**)"
interruptMode: tool-only
---

# Plugin Trait Init

- `Plugin::init()` 接收 `&PluginContext`（请求级上下文）和 `Arc<PluginHandle>`（插件服务句柄）
- `PluginHandle` 从 `HostApi::register(plugin_id, config)` 获取，绑定插件身份与配置
- 用 `handle` 参数执行平台操作。用 `ctx` 参数获取 trace_id、query_id 等请求级信息
- **禁止** 在插件内部状态中存储 `PluginHandle`；通过 `init` 参数或 `PluginContext` 访问
