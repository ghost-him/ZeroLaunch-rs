---
description: PluginHandle 使用 — 插件必须通过 PluginHandle 访问平台能力，无对应方法时必须先添加到 PluginHandle
condition: "PluginHandle|HostApi::register"
scope: "tool:edit(src-tauri/src/**), tool:write(src-tauri/src/**), tool:edit(crates/plugin-api/src/**), tool:write(crates/plugin-api/src/**)"
interruptMode: tool-only
---

# PluginHandle 使用

- 插件 **必须** 通过 `PluginHandle`（从 `HostApi::register()` 获取）访问平台能力。可用方法列表见 `PluginHandle` 源码（`crates/plugin-api/src/host/plugin_handle.rs`）
- 如果某平台操作没有 `PluginHandle` 方法，**必须** 先添加到 `PluginHandle` 再使用
