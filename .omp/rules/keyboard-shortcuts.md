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

## 插件唤醒热键（前端处理，不注册 OS 全局热键）

- **OS 级快捷键只有一项职责：呼出/隐藏搜索栏**（`hotkey-config` 的 `open_search_bar`，含双击 Ctrl）。
  插件声明的唤醒热键（`PluginMetadata.hotkey` / `InstalledPluginInfo.hotkey`）**禁止** 注册为
  OS 全局热键——搜索栏未唤起时按它必须无效（窗口未激活，前端收不到按键）
- 插件唤醒热键在前端 `useKeyboardRouter.onKeyDown` 中匹配（`matchesKey`，与可配置键同格式契约），
  命中即调 `bridge_wake_plugin(plugin_id)`（后端 `SessionDispatcher::wake_plugin` 空查询进入
  全面板会话，载荷经 `session-state.panelContent` 推送）
- 热键表数据源：`plugin_list`（含 `enabled` + `hotkey`），窗口每次唤起（`onShowWindow`）与
  插件安装/卸载后刷新——热键仅在窗口内生效，无需实时推送
- 已在同一插件面板会话时不重复唤醒（放行给面板绑定）
- **后端权威校验**：`wake_plugin` 先校验插件启用态（`SessionDispatcher.is_plugin_enabled`，
  禁用插件即使前端热键表残留过期条目也不得被唤醒），且要求响应为 `CustomPanel` 且
  `keep_search_bar=false`——热键唤醒默认 = 完全插件模式 = 全页面接管；List/Empty 属违约
  直接报错；`keep_search_bar: true`（行内形态）属违约：debug 构建用 `debug_assert!` 强制
  panic 暴露（契约违约即宿主逻辑缺陷，快速定位），release 构建正常运行——按插件声明形态
  降级为 PluginPanel（保留搜索栏），与 `route_query` 的 keep_search_bar → 展示形态映射一致
- 宿主面板绑定与插件热键冲突时插件热键优先（窗口级全局唤醒语义）；插件作者应避免与宿主键冲突

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
