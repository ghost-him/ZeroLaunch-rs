---
description: 推送式回调模式 — register_callback 注册，DashMap 存储，start/stop_watching 管理生命周期，全局启停保留在 HostApi
condition: "register_callback|unregister_callback|start_watching|stop_watching|start_listening|stop_listening|DashMap"
scope: "tool:edit(crates/plugin-api/src/**), tool:write(crates/plugin-api/src/**), tool:edit(crates/platform-windows/src/**), tool:write(crates/platform-windows/src/**), tool:edit(src-tauri/src/sdk.rs), tool:write(src-tauri/src/sdk.rs)"
---

# 推送式回调模式

对于向应用推送事件的服务（`HotkeyManager`、`InstallationMonitor`、`FocusMonitor`、`TimerManager`）：

- **正确**：通过 `register_callback(id, callback)` 注册回调，通过 `unregister_callback(id)` 取消注册
- **正确**：将回调存储在线程安全集合（`DashMap`）中，事件发生时依次调用所有回调
- **正确**：通过 `start_watching()` / `stop_watching()` 管理生命周期
- **正确**：回调注册/注销可以通过 `PluginHandle` 暴露，插件通过句柄注册自己的回调
- **正确**：`PluginHandle` 上的回调注册方法内部用 `plugin_id` 前缀化 callback ID，避免不同插件间的 ID 冲突
- **全局生命周期管理**（`start_listening`、`stop_listening`、`start_watching`、`stop_watching`）保留在 `HostApi` 上，插件只能注册/注销自己的回调，不能启停全局服务
