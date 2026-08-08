import { computed, onMounted, onUnmounted } from 'vue'
import { useSearchStore, type SessionMode } from '@/stores/search-store'
import { useConfigStore } from '@/stores/config-store'
import { dispatchKeyDown } from './keyboard/registry'
import type { KeyOpts } from './keyboard/types'

export function useKeyboardRouter() {
  const store = useSearchStore()
  const configStore = useConfigStore()

  const uiMode = computed<SessionMode>(() => store.sessionMode)

  /// Windows 菜单模式拦截：单独按下 Alt 会激活窗口菜单模式（WM_SYSKEYDOWN），
  /// 之后第一个按键的 keydown 会被系统当作菜单助记符吞掉（只有 keyup 到达），
  /// 表现为"按过 Alt 后第一个字母无法输入"。
  /// 在捕获阶段对 Alt 键事件 preventDefault，阻止菜单模式激活。
  /// 系统级 Alt+Space 热键由 RegisterHotKey 在 DOM 分发之前处理，不受影响；
  /// Alt+字母 组合（插件声明式按键绑定）的字母 keydown 仍正常分发，不受影响。
  /// Ctrl+Alt（AltGr）组合不拦截，避免破坏 AltGr 字符输入。
  function onAltKeyCapture(e: KeyboardEvent) {
    if (e.ctrlKey) return
    if (e.key === 'Alt' || e.code === 'AltLeft' || e.code === 'AltRight') {
      e.preventDefault()
    }
  }

  function onKeyDown(e: KeyboardEvent) {
    // 键盘解释选项来自 window-behavior-config 设置（缺省 false）
    const wb = (configStore.settings['window-behavior-config'] as Record<string, boolean> | undefined)
    const opts: KeyOpts = {
      spaceIsEnter: wb?.space_is_enter ?? false,
      escHideWindowPriority: wb?.is_esc_hide_window_priority ?? false,
    }
    dispatchKeyDown(e, store, opts)
  }

  onMounted(() => {
    document.addEventListener('keydown', onAltKeyCapture, true)
    document.addEventListener('keyup', onAltKeyCapture, true)
    document.addEventListener('keydown', onKeyDown)
  })
  onUnmounted(() => {
    document.removeEventListener('keydown', onAltKeyCapture, true)
    document.removeEventListener('keyup', onAltKeyCapture, true)
    document.removeEventListener('keydown', onKeyDown)
  })

  return { uiMode }
}
