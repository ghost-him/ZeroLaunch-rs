import { computed, onMounted, onUnmounted } from 'vue'
import { useSearchStore } from '@/stores/search-store'
import { useConfigStore } from '@/stores/config-store'
import { handleSearchModeKey } from './keyboard/searchHandler'
import { handleInlineParamKey } from './keyboard/inlineParamHandler'
import { handleParamPanelKey } from './keyboard/paramPanelHandler'
import { handleInlinePluginKey } from './keyboard/inlinePluginHandler'
import { handleFullPagePluginKey } from './keyboard/fullPagePluginHandler'

export type UIMode = 'none' | 'search' | 'inline_param' | 'param_panel' | 'inline_plugin' | 'full_page_plugin'

export function useKeyboardRouter() {
  const store = useSearchStore()
  const configStore = useConfigStore()

  const uiMode = computed<UIMode>(() => {
    if (store.inlineParamState) return 'inline_param'
    if (store.paramPanelState) return 'param_panel'
    if (store.sessionMode === 'full_page_plugin') return 'full_page_plugin'
    if (store.sessionMode === 'inline_plugin') return 'inline_plugin'
    if (store.sessionMode === 'search') return 'search'
    return 'none'
  })

  function onKeyDown(e: KeyboardEvent) {
    // Alt+Space 始终保留给系统（唤醒/隐藏窗口）
    if (e.altKey && e.code === 'Space') return

    const wb = (configStore.settings['window-behavior'] as Record<string, boolean> | undefined)
    const spaceIsEnter = wb?.space_is_enter ?? false

    switch (uiMode.value) {
      case 'search':
        handleSearchModeKey(e, store, spaceIsEnter)
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
        break
    }
  }

  onMounted(() => document.addEventListener('keydown', onKeyDown))
  onUnmounted(() => document.removeEventListener('keydown', onKeyDown))

  return { uiMode }
}
