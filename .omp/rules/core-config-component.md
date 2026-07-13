---
description: 核心配置组件模式 — apply_settings 禁止 spawn async，副作用必须在 on_settings_changed 中
condition: "on_settings_changed|apply_settings|spawn"
scope: "tool:edit(src-tauri/src/builtin_plugin/**), tool:write(src-tauri/src/builtin_plugin/**)"
interruptMode: tool-only
---

# 核心配置组件模式（builtin_plugin/config/ 中的组件）

- 核心配置组件（HotkeyConfig、AppearanceConfig 等）可使用更复杂的内部状态
- 可持有 `Arc<HostApi>` 用于在 `on_settings_changed()` 中调用平台服务
- `on_settings_changed()` 中 **允许** spawn async task 执行副作用（如注册热键、启动监控）
- **禁止** 在 `apply_settings()` 中 spawn async task。副作用 **必须** 在 `on_settings_changed()` 中
