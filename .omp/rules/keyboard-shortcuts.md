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

## 可配置键分发（configKey 别名）

- 用户可配置的宿主键（如 window-behavior-config 的 `move_up_key`/`move_down_key`）
  **必须** 通过 `HostKeyBinding.configKey` 在 `hostPanels.ts` 绑定表中声明，
  `registry.ts` 仅解释执行、**禁止** 在 dispatcher 内命令式构造配置键绑定
- 语义：`configKey` 是**别名并存**——配置键非空时与静态 `key` 同时生效（任一命中即接管）；
  空串 = 未设置，仅静态 key 生效
- 面板作用域由绑定声明位置决定：仅配置键驱动的绑定省略 `key`（如参数面板的上下选择），
  方向键放行给输入框做编辑；搜索面板的方向键绑定带 `configKey`（方向键与配置键并存）
- 配置键字符串格式契约以 `HotkeyField` 录制端为权威（e.code 规范化、布局无关）：
  `matchesKey` 必须兼容 `'Space'` 与单字符主键的 `e.key`/`e.code` 双通道匹配
- 配置键校验（修饰键必需、保留键拒绝）由后端 `validate_settings` 承担，
  前端不重复校验
