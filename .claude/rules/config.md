---
paths:
  - "src-tauri/src/core/config/**"
  - "src-tauri/src/commands/config_file.rs"
---

# Config 系统规范

## ConfigManager 的 save/load 流水线

`ConfigManager::apply_settings()` 按以下固定顺序执行，不可重排、不可跳过、不可合并：

1. `validate_settings(&settings)?` — 纯校验，不得修改状态
2. `apply_settings(settings)` — 仅写入内部 `RwLock`
3. `on_settings_changed()` — 副作用（重建服务、注册回调）
4. 通过广播通道发布 `ConfigEvent`
5. `save_to_storage()` — 持久化到本地 JSON，然后可选远程同步

如果校验（步骤 1）失败，后续步骤 **不得** 执行。
广播事件（步骤 4）**无论** 持久化成功与否都会触发。

## Configurable Trait 实现

- `apply_settings(&self, settings)` 接收 `&self`（不可变引用）。用内部 `RwLock` 实现内部可变性。**禁止** 用 `&mut self`
- `get_settings()` 返回 `serde_json::Value`。返回当前配置的 JSON 格式。**禁止** 返回内部 Rust 类型
- `setting_schema()` 返回 `Vec<SettingDefinition>`。此方法驱动前端设置 UI。前端接收到的就是此方法返回的内容
- `default_enabled()` 返回 `bool`。默认为 `true`。对可选/实验性组件覆盖
- `get_default_settings()` 返回 `serde_json::Value`。供 `config_reset_settings` 调用

## ConfigAction 用于保存前测试

- 如果副作用必须决定配置能否保存（如 WebDAV 连通性测试）：使用 `ConfigAction`。**禁止** 将此类测试嵌入 `validate_settings` 或 `apply_settings`
- **正确**：通过 `config_actions()` → `Vec<ConfigActionDef>` 声明，在 `execute_config_action(&self, action: &str)` → `Result<serde_json::Value, String>` 中实现
- **正确**：前端将 `config_execute_action` 作为单独的用户触发操作调用，与保存流程解耦
- **禁止**：将网络调用、文件系统测试或外部服务检查放入 `validate_settings` 或 `apply_settings`

## 组件粒度

- 按 **功能域** 拆分配置组件。每个组件处理 **一个** 关注点
- **正确**：`AppearanceConfigComponent`（主题、语言）、`StorageConfigComponent`（后端、路径）、`HotkeyConfigComponent`（快捷键）、`InstallationMonitorConfigComponent`（监控设置）
- **错误**：`AppConfigComponent` 或 `UIConfigComponent` 将所有设置打包成一个巨无霸组件
- 每个组件拥有自己的 `component_id`、自己的 settings JSON、自己的 schema。**禁止** 跨组件共享 settings 对象

## 组件位置

- 核心配置组件（非插件）：**必须** 放在 `core/config/components/`
- 插件专属配置：属于插件的 `Configurable` impl。**禁止** 将插件配置放在 `core/config/components/`
- 配置组件实现 `Configurable` trait。**禁止** 在配置组件中 import `plugin/` 或 `plugin_system/` 的类型
- 配置组件可持有 `Arc<HostApi>` 以在 `on_settings_changed()` 中调用平台服务

## 存储分离

- `ConfigStore` 处理本地 JSON 文件持久化。远程同步（WebDAV）是可选的，单独处理
- 本地持久化 **必须** 独立于远程同步而成功。**禁止** 让远程同步失败阻塞本地保存或导致配置回滚
