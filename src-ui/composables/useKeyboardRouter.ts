import { computed, onMounted, onUnmounted } from 'vue'
import { useSearchStore, type SessionMode } from '@/stores/search-store'
import { useConfigStore } from '@/stores/config-store'
import { dispatchKeyDown } from './keyboard/registry'
import type { KeyOpts } from './keyboard/types'

export function useKeyboardRouter() {
  const store = useSearchStore()
  const configStore = useConfigStore()

  const uiMode = computed<SessionMode>(() => store.sessionMode)

  function onKeyDown(e: KeyboardEvent) {
    // 键盘解释选项来自 window-behavior-config 设置（缺省 false）
    const wb = (configStore.settings['window-behavior-config'] as Record<string, boolean> | undefined)
    const opts: KeyOpts = {
      spaceIsEnter: wb?.space_is_enter ?? false,
      escHideWindowPriority: wb?.is_esc_hide_window_priority ?? false,
    }
    dispatchKeyDown(e, store, opts)
  }

  onMounted(() => document.addEventListener('keydown', onKeyDown))
  onUnmounted(() => document.removeEventListener('keydown', onKeyDown))

  return { uiMode }
}
