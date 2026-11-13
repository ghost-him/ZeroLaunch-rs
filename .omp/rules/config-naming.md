---
description: 配置组件命名规范 — component_id 用 kebab-case + -config 后缀，setting key 用 snake_case
condition: "component_id|kebab|snake_case|setting_key"
scope: "tool:edit(crates/zerolaunch-rs/src/builtin_plugin/**), tool:write(crates/zerolaunch-rs/src/builtin_plugin/**), tool:edit(crates/zerolaunch-rs/src/core/**), tool:write(crates/zerolaunch-rs/src/core/**), tool:edit(app/src/settings/**), tool:write(app/src/settings/**)"
---

# 配置组件命名规范

- `component_id`：**必须** 使用 kebab-case，**必须** 以 `-config` 后缀结尾（如 `"hotkey-config"`、`"window-behavior-config"`、`"appearance-config"`）。此后缀用于标识该组件为配置组件，与 `-source`（DataSource）、`-executor`（ActionExecutor）等命名惯例保持一致
- Setting JSON key：**必须** 使用 snake_case（如 `"open_search_bar"`、`"is_esc_hide_window_priority"`）
- GPUI 设置页直接经 `ConfigManager` 读取当前值，key 名与后端一致
- 新增/重命名 key 时，schema、持久化结构与设置页视图 **同一 commit** 中同步修改
- GPUI 设置页需要定位组件时，组件 id 以字面量出现（如 `'help-agent-config'`）。字面量 **必须** 注释互指后端常量（`core/<domain>/settings.rs` 的 `*_CONFIG_ID`，如 `HELP_AGENT_CONFIG_ID`）；后端改 id 时**同一 commit** 同步视图字面量。
