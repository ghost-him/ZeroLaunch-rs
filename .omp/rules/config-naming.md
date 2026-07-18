---
description: 配置组件命名规范 — component_id 用 kebab-case + -config 后缀，setting key 用 snake_case，前后端同一 commit 同步
condition: "component_id|kebab|snake_case|setting_key"
scope: "tool:edit(*.rs), tool:write(*.rs), tool:edit(*.ts), tool:write(*.ts)"
---

# 配置组件命名规范

- `component_id`：**必须** 使用 kebab-case，**必须** 以 `-config` 后缀结尾（如 `"hotkey-config"`、`"window-behavior-config"`、`"appearance-config"`）。此后缀用于标识该组件为配置组件，与 `-source`（DataSource）、`-executor`（ActionExecutor）等命名惯例保持一致
- Setting JSON key：**必须** 使用 snake_case（如 `"open_search_bar"`、`"is_esc_hide_window_priority"`）
- 前端通过 `configStore.settings[component_id][setting_key]` 读取，key 名与后端一致
- 前后端 key 名 **必须** 保持一致。新增/重命名 key 时，前后端 **同一 commit** 中同步修改
