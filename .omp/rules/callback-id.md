---
description: PluginHandle 回调 ID 规范 — ID 自动被 plugin_id 前缀化，禁止手动添加前缀，注销在 on_settings_changed 或 drop 时
condition: "register_hotkey_callback|register_callback|unregister_callback|callback_id"
scope: "tool:edit(src-tauri/src/**), tool:write(src-tauri/src/**), tool:edit(crates/plugin-api/src/**), tool:write(crates/plugin-api/src/**)"
interruptMode: tool-only
---

# PluginHandle 回调 ID 规范

- 通过 PluginHandle 注册回调时，ID 自动被 plugin_id 前缀化（如 `"core:search_bar_toggle"`），**必须** 避免手动添加前缀
- 回调注销 **必须** 在 `on_settings_changed()` 或组件 drop 时执行
