import type { useSearchStore } from '@/stores/search-store'

/** 键盘解释器的统一意图：宿主面板 handler / 插件动作翻译后的可执行语义。 */
export type KeyIntent =
  | { kind: 'query'; text?: string; confirm: boolean }
  | { kind: 'confirm'; actionId?: string }
  | { kind: 'back' }
  | { kind: 'hide' }
  | { kind: 'gotoPanel'; panelId: string }
  | { kind: 'custom'; action: string; args: unknown }
  | { kind: 'local'; run: () => void }

/** 宿主面板按键绑定：静态 key 与 configKey 配置的键（非空时）任一命中即触发 handler。
 *  key 可省略 —— 仅由用户配置键驱动的绑定（如参数面板的上下选择）不设静态键。 */
export interface HostKeyBinding {
  /** 静态键（如 "ArrowDown"）；仅配置键驱动的绑定可省略。 */
  key?: string
  /** 可选：绑定同时响应用户配置的键（KeyOpts 字段名）。配置值非空时与静态 key 并存（别名），空串 = 未设置。 */
  configKey?: keyof Pick<KeyOpts, 'moveUpKey' | 'moveDownKey'>
  handler: (e: KeyboardEvent, store: ReturnType<typeof useSearchStore>, opts: KeyOpts) => KeyIntent | null
}

/** 宿主键盘解释选项（来自 window-behavior-config 设置：布尔项缺省 false；
 *  按键项缺省 Ctrl+K/Ctrl+J，空串 = 未设置，仅保留方向键）。 */
export interface KeyOpts {
  spaceIsEnter: boolean
  escHideWindowPriority: boolean
  /** 配置的向上/向下选择键（Hotkey 字符串如 "Ctrl+K"，空串 = 未设置，仅保留方向键）。 */
  moveUpKey?: string
  moveDownKey?: string
}
