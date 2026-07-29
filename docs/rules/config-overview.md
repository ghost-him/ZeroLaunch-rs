---
description: 配置系统总览：ConfigManager save/load 流水线、组件粒度、存储分离
condition: ".*"
scope: "tool:read(src-tauri/src/core/config/**), tool:edit(src-tauri/src/core/config/**), tool:write(src-tauri/src/core/config/**), tool:read(src-tauri/src/commands/config_file.rs), tool:edit(src-tauri/src/commands/config_file.rs), tool:write(src-tauri/src/commands/config_file.rs), tool:read(crates/plugin-api/src/config/**), tool:edit(crates/plugin-api/src/config/**), tool:write(crates/plugin-api/src/config/**), tool:read(src-tauri/src/builtin_plugin/**), tool:edit(src-tauri/src/builtin_plugin/**), tool:write(src-tauri/src/builtin_plugin/**)"
---

# Config 系统总览

## ConfigManager 的 save/load 流水线

`ConfigManager::apply_settings()` 按以下固定顺序执行，不可重排、不可跳过、不可合并：

1. `validate_settings(&settings)?` — 纯校验，不得修改状态
   - **禁止**：修改 `self` 内部状态、执行文件/网络 I/O、调用 HostApi、注册/注销回调、spawn async task
   - **允许**：检查字段值范围、验证必填项、检查字段间约束、返回 `Err(ConfigError)` 拒绝无效输入
2. `apply_settings(settings)` — 仅写入内部 `RwLock`
   - **禁止**：重建外部服务、启动/停止监听器、注册热键、调用 HostApi、spawn async task、执行 I/O
   - **允许**：反序列化 settings JSON、写入 `self.settings`（或等价的内部 RwLock）
3. `on_settings_changed()` — 副作用（重建服务、注册回调）
   - **禁止**：修改配置值（此时配置已生效，再改会造成不一致）
   - **允许**：spawn async task 注册热键、重启监听器、发送通知等
4. 通过广播通道发布 `ConfigEvent`
5. `save_to_storage()` — 持久化到本地 JSON，然后可选远程同步

双通道事件总线：`ConfigManager` 同时是 `PluginRuntimeEvent` 的监听端，接收来自 `PluginManager` 的插件生命周期事件（加载/卸载），自动同步注册或解注册对应组件配置，再通过 `ConfigEvent` 通知 `SessionRouter` 重建管道。三层解耦：`PluginManager → PluginRuntimeEvent → ConfigManager → ConfigEvent → SessionRouter`。

如果校验（步骤 1）失败，后续步骤 **不得** 执行。
广播事件（步骤 4）**无论** 持久化成功与否都会触发。

## 组件粒度

- 按 **功能域** 拆分配置组件。每个组件处理 **一个** 关注点
- **正确**：`AppearanceConfigComponent`（主题、语言）、`StorageConfigComponent`（后端、路径）、`HotkeyConfigComponent`（快捷键）、`InstallationMonitorConfigComponent`（监控设置）
- **错误**：`AppConfigComponent` 或 `UIConfigComponent` 将所有设置打包成一个巨无霸组件
- 每个组件拥有自己的 `component_id`、自己的 settings JSON、自己的 schema。**禁止** 跨组件共享 settings 对象

## 组件位置

- 核心配置组件（非插件）和插件专属配置的放置规则见 `.omp/AGENTS.md` 的新文件放置决策树
- 配置组件实现 `Configurable` trait，可持有 `Arc<HostApi>` 以在 `on_settings_changed()` 中调用平台服务

## 存储分离

- `ConfigStore` 处理本地 JSON 文件持久化。远程同步（WebDAV）是可选的，单独处理
- 本地持久化 **必须** 独立于远程同步而成功
