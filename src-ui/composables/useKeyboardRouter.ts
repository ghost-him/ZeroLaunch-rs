import { computed, onMounted, onUnmounted } from 'vue'
import { useSearchStore, type SessionMode } from '@/stores/search-store'
import { useConfigStore } from '@/stores/config-store'
import { handleSearchModeKey } from './keyboard/searchHandler'
import { handleInlineParamKey } from './keyboard/inlineParamHandler'
import { handleParamPanelKey } from './keyboard/paramPanelHandler'
import { handleInlinePluginKey } from './keyboard/inlinePluginHandler'
import { handleFullPagePluginKey } from './keyboard/fullPagePluginHandler'

export function useKeyboardRouter() {
  const store = useSearchStore()
  const configStore = useConfigStore()

  const uiMode = computed<SessionMode>(() => store.sessionMode)

  function onKeyDown(e: KeyboardEvent) {
    // Alt+Space 始终保留给系统（唤醒/隐藏窗口）
    if (e.altKey && e.code === 'Space') return

    const wb = (configStore.settings['window-behavior-config'] as Record<string, boolean> | undefined)
    const spaceIsEnter = wb?.space_is_enter ?? false

    switch (uiMode.value) {
      case 'search':
        handleSearchModeKey(e, store, { spaceIsEnter, isEscHideWindowPriority: wb?.is_esc_hide_window_priority ?? false })
        break
      case 'inline_param':
        handleInlineParamKey(e, store)
        break
      case 'param_panel':
        handleParamPanelKey(e, store)
        break
      case 'inline_plugin':
        handleInlinePluginKey(e, store)
        break
      case 'full_page_plugin':
        handleFullPagePluginKey(e, store)
        break
      case 'none':
        if (e.key === 'Escape') {
          e.preventDefault()
          store.hideWindow()
        }
        break
    }
  }

  onMounted(() => document.addEventListener('keydown', onKeyDown))
  onUnmounted(() => document.removeEventListener('keydown', onKeyDown))

  return { uiMode }
}
