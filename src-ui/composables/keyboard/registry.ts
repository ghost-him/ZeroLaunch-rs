import type { useSearchStore } from '@/stores/search-store'
import { bridgeConfirm } from '@/bridge/commands'
import type { PanelKeyAction } from '@/bridge/contract'
import { hostPanels, type HostPanelId } from './hostPanels'
import type { KeyIntent, KeyOpts } from './types'

/// 系统保留键：Alt+Space（唤醒/隐藏窗口）一律不拦截。
function isSystemReserved(e: KeyboardEvent): boolean {
  return e.altKey && e.code === 'Space'
}

/// 按键说明符匹配（如 "Enter" / "Ctrl+Enter" / "Shift+Tab" / "a"）：
/// 拆 '+'；修饰键 Ctrl/Shift/Alt/Meta 精确匹配（未声明的修饰键必须为 false）。
/// 主键：单字符按 e.key 忽略大小写比对；特殊名（Enter/Escape/Tab/Backspace/Home/End/ArrowUp/ArrowDown 等）按 e.key 精确比对；
/// "Digit" 主键按 e.code 前缀比对（宿主 Ctrl+1..9 快捷动作需区分主键盘数字区与数字小键盘）。
export function matchesKey(e: KeyboardEvent, spec: string): boolean {
  const parts = spec.split('+')
  const main = parts[parts.length - 1]
  const mods = parts.slice(0, -1)

  if (mods.includes('Ctrl') !== e.ctrlKey) return false
  if (mods.includes('Shift') !== e.shiftKey) return false
  if (mods.includes('Alt') !== e.altKey) return false
  if (mods.includes('Meta') !== e.metaKey) return false

  if (main === 'Digit') return e.code.startsWith('Digit')
  if (main.length === 1) return e.key.toLowerCase() === main.toLowerCase()
  return e.key === main
}

/// 插件面板动作 → 宿主意图（confirm 即宿主 Enter 标准语义，走 store.confirmQuery）。
function translateAction(action: PanelKeyAction, store: ReturnType<typeof useSearchStore>): KeyIntent {
  switch (action.kind) {
    case 'confirm':
      return { kind: 'local', run: () => store.confirmQuery() }
    case 'executeAction':
      return { kind: 'confirm', actionId: action.actionId ?? undefined }
    case 'goBack':
      return { kind: 'back' }
    case 'gotoPanel':
      return { kind: 'gotoPanel', panelId: action.panelId }
    case 'custom':
      return { kind: 'custom', action: action.action, args: action.args }
  }
}

/// 执行宿主意图：查询/确认/返回/隐藏/面板动作转发/本地闭包。
export function applyIntent(intent: KeyIntent, store: ReturnType<typeof useSearchStore>) {
  switch (intent.kind) {
    case 'query':
      void store.doQuery(intent.text ?? store.query, intent.confirm)
      break
    case 'confirm':
      store.doConfirm(undefined, intent.actionId)
      break
    case 'back':
      store.back()
      break
    case 'hide':
      store.hideWindow()
      break
    case 'gotoPanel':
      void bridgeConfirm({
        kind: 'pluginAction',
        pluginId: store.currentPluginId ?? '',
        action: 'goto_panel',
        args: { panelId: intent.panelId },
        generation: store.currentGeneration,
      }).catch((e) => console.warn('[keyboard] gotoPanel 失败:', e))
      break
    case 'custom':
      void bridgeConfirm({
        kind: 'pluginAction',
        pluginId: store.currentPluginId ?? '',
        action: intent.action,
        args: intent.args,
        generation: store.currentGeneration,
      }).catch((e) => console.warn('[keyboard] 面板动作失败:', e))
      break
    case 'local':
      intent.run()
      break
  }
}

/// 插件按键转发载荷：原始按键信息经面板动作通道回插件解释。
/// 插件面板分发：声明式按键绑定命中 → 按动作执行；未命中 → 放行（交还浏览器/输入框）。
/// 设计语义：声明即接管 —— 插件必须声明全部所需按键（Enter/Escape 等），
/// 未声明的键宿主不解释、不兜底，全部放行（插件全权决定自己的按键行为，
/// 状态转换经显式动作 Confirm/GoBack/GotoPanel/Custom 触发）。
function dispatchPluginPanel(
  e: KeyboardEvent,
  store: ReturnType<typeof useSearchStore>,
  _opts: KeyOpts,
) {
  for (const binding of store.panelInteraction?.bindings ?? []) {
    if (matchesKey(e, binding.key)) {
      e.preventDefault()
      applyIntent(translateAction(binding.action, store), store)
      return
    }
  }
  // 未命中绑定：放行（不拦截、不 preventDefault）
}

/// 宿主面板分发：遍历绑定找 matchesKey，命中且 handler 产出意图 → preventDefault + 执行；否则放行。
function dispatchHost(
  e: KeyboardEvent,
  store: ReturnType<typeof useSearchStore>,
  opts: KeyOpts,
  panelId: HostPanelId,
) {
  for (const binding of hostPanels[panelId].bindings) {
    if (matchesKey(e, binding.key)) {
      const intent = binding.handler(e, store, opts)
      if (intent !== null) {
        e.preventDefault()
        applyIntent(intent, store)
      }
      return
    }
  }
  // 未命中绑定或 handler 放行：不拦截（交给输入框/默认行为）
}

/// 键盘分发入口：按会话模式路由到对应面板解释器（Alt+Space 系统保留键除外）。
export function dispatchKeyDown(e: KeyboardEvent, store: ReturnType<typeof useSearchStore>, opts: KeyOpts) {
  if (isSystemReserved(e)) return

  switch (store.sessionMode) {
    case 'search':
    case 'none':
      dispatchHost(e, store, opts, 'default_search')
      return
    case 'inline_param':
      dispatchHost(e, store, opts, 'inline_param')
      return
    case 'param_panel':
      dispatchHost(e, store, opts, 'param_panel')
      return
    case 'plugin_panel':
    case 'plugin_immersive':
      dispatchPluginPanel(e, store, opts)
      return
  }
}
