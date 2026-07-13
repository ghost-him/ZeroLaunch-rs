---
description: 键盘快捷键规范 — 搜索窗口键盘处理集中在 useKeyboardRouter，禁止子组件添加全局 keydown
condition: "useKeyboardRouter|keydown|addEventListener.*key|keyboard/"
scope: "tool:edit(*.vue), tool:edit(*.ts), tool:write(*.vue), tool:write(*.ts)"
interruptMode: tool-only
---

# 键盘快捷键

- 搜索窗口的键盘处理集中在 `composables/useKeyboardRouter.ts`，具体处理器在 `composables/keyboard/` 子目录（`searchHandler.ts`、`inlineParamHandler.ts`、`inlinePluginHandler.ts`、`fullPagePluginHandler.ts`、`paramPanelHandler.ts`）
- **禁止** 在子组件中添加全局 `keydown` 监听器。由 composable 统一管理
- 插件面板激活时（immersive mode），搜索快捷键 **必须** 被抑制
