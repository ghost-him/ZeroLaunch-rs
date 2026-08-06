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

/** 宿主面板按键绑定：key 经 matchesKey 匹配，命中后调用 handler 产出意图。 */
export interface HostKeyBinding {
  key: string
  handler: (e: KeyboardEvent, store: ReturnType<typeof useSearchStore>, opts: KeyOpts) => KeyIntent | null
}

/** 宿主键盘解释选项（来自 window-behavior-config 设置，缺省 false）。 */
export interface KeyOpts {
  spaceIsEnter: boolean
  escHideWindowPriority: boolean
}
