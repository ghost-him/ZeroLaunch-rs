---
description: Pre-mount 初始化顺序 — 外观主题必须在 app.mount() 前加载，防止白屏闪烁
condition: "app\\.mount|createApp|createPinia|loadFromBackend|app\\.use"
scope: "tool:edit(*.ts), tool:edit(*.vue), tool:write(*.ts), tool:write(*.vue)"
---

# Pre-mount 初始化

- 外观主题 **必须** 在 app.mount() 之前加载并应用（防止白屏闪烁）
- 顺序：createPinia → createApp → useThemeStore().loadFromBackend() → app.mount()
