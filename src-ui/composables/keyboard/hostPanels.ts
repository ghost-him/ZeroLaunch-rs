import type { useSearchStore } from '@/stores/search-store'
import type { HostKeyBinding, KeyIntent } from './types'

export type HostPanelId = 'default_search' | 'inline_param' | 'param_panel'

/// 向下移动选中项意图（方向键与配置键共用语义）。
export function moveDownIntent(_e: KeyboardEvent, store: ReturnType<typeof useSearchStore>): KeyIntent {
  return { kind: 'local', run: () => store.selectNext() }
}

/// 向上移动选中项意图（方向键与配置键共用语义）。
export function moveUpIntent(_e: KeyboardEvent, store: ReturnType<typeof useSearchStore>): KeyIntent {
  return { kind: 'local', run: () => store.selectPrev() }
}

/// 循环切换选中动作（Shift 反向；与旧 searchHandler Tab 行为一致）。
function cycleSelectedAction(e: KeyboardEvent, store: ReturnType<typeof useSearchStore>): KeyIntent | null {
  const item = store.selectedItem
  if (!item || item.actions.length === 0) return null
  const delta = e.shiftKey ? -1 : 1
  const n = item.actions.length
  return {
    kind: 'local',
    run: () => {
      store.selectedActionIndex = (store.selectedActionIndex + delta + n) % n
    },
  }
}

/// Ctrl+1..9 快捷执行对应动作（仅主键盘数字区：e.code 以 Digit 开头，排除数字小键盘）。
function quickAction(e: KeyboardEvent, store: ReturnType<typeof useSearchStore>): KeyIntent | null {
  if (!e.code.startsWith('Digit')) return null
  const num = parseInt(e.code.slice('Digit'.length), 10)
  if (num < 1 || num > 9) return null
  const item = store.selectedItem
  if (!item || num - 1 >= item.actions.length) return null
  const action = item.actions[num - 1]
  return { kind: 'local', run: () => store.doConfirm(undefined, action.id) }
}

export const hostPanels: Record<HostPanelId, { bindings: HostKeyBinding[] }> = {
  // 搜索模式（sessionMode 'search' 与 'none'）：行为与旧 searchHandler 逐键一致
  default_search: {
    bindings: [
      // configKey：方向键与用户配置的上下选择键（move_up_key/move_down_key）别名并存
      { key: 'ArrowDown', configKey: 'moveDownKey', handler: moveDownIntent },
      { key: 'ArrowUp', configKey: 'moveUpKey', handler: moveUpIntent },
      { key: 'Enter', handler: () => ({ kind: 'confirm' }) },
      {
        key: ' ',
        // 空格：spaceIsEnter 时确认查询，否则放行给输入框（由 bridge_query 后端判断是否触发 inline_param）
        handler: (_e, _store, opts) => (opts.spaceIsEnter ? { kind: 'confirm' } : null),
      },
      {
        key: 'Escape',
        handler: (_e, store, opts) =>
          opts.escHideWindowPriority || store.query === '' ? { kind: 'hide' } : { kind: 'back' },
      },
      { key: 'Tab', handler: cycleSelectedAction },
      { key: 'Shift+Tab', handler: cycleSelectedAction },
      { key: 'Home', handler: (_e, store) => ({ kind: 'local', run: () => { store.selectedIndex = 0 } }) },
      {
        key: 'End',
        handler: (_e, store) => ({
          kind: 'local',
          run: () => { store.selectedIndex = Math.max(store.results.length - 1, 0) },
        }),
      },
      { key: 'Ctrl+Digit', handler: quickAction },
      // 旧实现将 Meta 与 Ctrl 等价对待（Mac 习惯），保留
      { key: 'Meta+Digit', handler: quickAction },
    ],
  },

  // 行内参数模式：行为与旧 inlineParamHandler 逐键一致
  inline_param: {
    bindings: [
      // 仅用户配置的上下选择键（Ctrl+K/J）生效；方向键放行给参数输入框做光标编辑
      { configKey: 'moveDownKey', handler: moveDownIntent },
      { configKey: 'moveUpKey', handler: moveUpIntent },
      { key: 'Enter', handler: (_e, store) => ({ kind: 'local', run: () => store.confirmInlineParam() }) },
      { key: 'Escape', handler: (_e, store) => ({ kind: 'local', run: () => store.exitInlineParamMode() }) },
      {
        key: 'Backspace',
        // 参数输入为空时退出行内模式；否则放行（正常删除字符）
        handler: (_e, store) =>
          store.inlineParamState?.paramInput === ''
            ? { kind: 'local', run: () => store.exitInlineParamMode() }
            : null,
      },
    ],
  },

  // 参数面板模式：行为与旧 paramPanelHandler 逐键一致
  param_panel: {
    bindings: [
      // 仅用户配置的上下选择键（Ctrl+K/J）生效；方向键放行给参数输入框做光标编辑
      { configKey: 'moveDownKey', handler: moveDownIntent },
      { configKey: 'moveUpKey', handler: moveUpIntent },
      {
        key: 'Enter',
        // 末字段确认，否则聚焦下一字段
        handler: (_e, store) => {
          const state = store.paramPanelState
          if (state && state.focusedFieldIndex < state.fields.length - 1) {
            return { kind: 'local', run: () => store.paramPanelFocusNext() }
          }
          return { kind: 'local', run: () => store.confirmParamPanel() }
        },
      },
      { key: 'Escape', handler: (_e, store) => ({ kind: 'local', run: () => store.exitParamPanelMode() }) },
      { key: 'Tab', handler: (_e, store) => ({ kind: 'local', run: () => store.paramPanelFocusNext() }) },
      { key: 'Shift+Tab', handler: (_e, store) => ({ kind: 'local', run: () => store.paramPanelFocusPrev() }) },
    ],
  },
}
