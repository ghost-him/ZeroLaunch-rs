---
description: 键盘快捷键规范 — 搜索窗口键盘处理集中在 useKeyboardRouter，禁止子组件添加全局 keydown
condition: "useKeyboardRouter|keydown|addEventListener.*key|keyboard/"
scope: "tool:edit(*.vue), tool:edit(*.ts), tool:write(*.vue), tool:write(*.ts)"
---

# 键盘快捷键

- 搜索窗口的键盘处理集中在 `composables/useKeyboardRouter.ts`：宿主面板由
  `composables/keyboard/hostPanels.ts` 提供代码 keymap（default_search / inline_param /
  param_panel），插件面板由声明式按键绑定（`PanelInteraction.bindings`，随 session-state
  事件下发，声明即接管、未声明即放行、宿主不兜底）驱动，
  统一经 `composables/keyboard/registry.ts` 的 `dispatchKeyDown` 解释执行
- **禁止** 在子组件中添加全局 `keydown` 监听器。由 composable 统一管理
- 插件面板激活时（immersive mode），搜索快捷键 **必须** 被抑制
