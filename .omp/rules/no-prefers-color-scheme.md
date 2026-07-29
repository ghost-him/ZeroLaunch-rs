---
description: 禁止使用 prefers-color-scheme 媒体查询 — 暗色模式通过 html.dark class 切换
condition: "prefers-color-scheme"
scope: "tool:edit(*.css), tool:edit(*.vue), tool:edit(*.ts), tool:write(*.css), tool:write(*.vue), tool:write(*.ts)"
---

你使用了 `@media (prefers-color-scheme)` 媒体查询。本项目的暗色模式通过 `html.dark` class 切换，禁止使用 `prefers-color-scheme`。

- **允许的例外**：JS `window.matchMedia('(prefers-color-scheme: dark)')` 可用于检测 OS 主题偏好以支持 "system" 模式（`theme-store.ts`）。此检测仅影响 `naiveTheme` 选择，不直接操作 CSS 变量。长期方案应由后端 IPC 读取 Windows 主题设置

外观配置由后端 `appearance` 组件管理：
- 后端设置变更 → 前端 `applyAppearanceSettings()` 更新 CSS 变量 → 组件自动响应
- `styles/variables.css` 定义所有 CSS 变量的静态默认值
- 暗色模式通过 `html.dark` class 切换，颜色使用 CSS 变量（如 `var(--text-primary)`）
