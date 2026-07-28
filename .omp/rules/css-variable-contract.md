---
description: CSS 变量契约 — 外观由后端 appearance 组件管理，通过 setProperty 操作，暗色模式用 html.dark class 切换
condition: "var\\(--|setProperty|prefers-color-scheme|html\\.dark|applyAppearanceSettings"
scope: "tool:edit(*.vue), tool:edit(*.css), tool:edit(*.ts), tool:write(*.vue), tool:write(*.css), tool:write(*.ts)"
---

# CSS 变量契约

- 所有外观配置（颜色、尺寸、字体）由后端 `appearance` 组件管理
- 后端设置变更 → 前端 `applyAppearanceSettings()` 更新 CSS 变量 → 组件自动响应
- `styles/variables.css` 定义所有 CSS 变量的 **静态默认值**。`styles/transitions.css` 定义全局过渡动画
- **必须** 通过 `setProperty('--var', val)` 操作 CSS 变量
- 暗色模式通过 `html.dark` class 切换。**禁止** 使用 CSS `@media (prefers-color-scheme)`
  - **允许的例外**：JS `window.matchMedia('(prefers-color-scheme: dark)')` 可用于检测 OS 主题偏好以支持 "system" 模式（`theme-store.ts`）。此检测仅影响 `naiveTheme` 选择，不直接操作 CSS 变量。长期方案应由后端 IPC 读取 Windows 主题设置
