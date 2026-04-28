import { watch } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { LogicalSize } from '@tauri-apps/api/dpi'
import { useSearchStore } from '../stores/search-store'

const BASE_WIDTH = 600
const SEARCH_BAR_HEIGHT = 56
const RESULT_ITEM_HEIGHT = 40
const FOOTER_HEIGHT = 32
const MAX_VISIBLE_RESULTS = 8
const MIN_CONTENT_HEIGHT = 100

export function useWindowResize() {
  const store = useSearchStore()
  const appWindow = getCurrentWindow()

  watch(
    () => ({
      mode: store.sessionMode,
      isIdle: store.isIdle,
      resultCount: store.results.length,
      keepSearchBar: store.keepSearchBar,
    }),
    async ({ mode, isIdle, resultCount, keepSearchBar }) => {
      let height: number

      if (isIdle && mode === 'none') {
        // Idle: compact, search bar only
        height = SEARCH_BAR_HEIGHT + 20
      } else if (mode === 'plugin' && !keepSearchBar) {
        // Immersive plugin mode: let plugin control the height
        height = 420
      } else if (mode === 'search' || (mode === 'plugin' && keepSearchBar)) {
        // Search results or plugin with search bar
        const visibleResults = Math.min(resultCount, MAX_VISIBLE_RESULTS)
        const resultsHeight = visibleResults > 0
          ? visibleResults * RESULT_ITEM_HEIGHT + 16
          : MIN_CONTENT_HEIGHT
        height = SEARCH_BAR_HEIGHT + resultsHeight + FOOTER_HEIGHT + 8
      } else {
        height = SEARCH_BAR_HEIGHT + 20
      }

      try {
        await appWindow.setSize(new LogicalSize(BASE_WIDTH, height))
      } catch (e) {
        console.warn('[useWindowResize] Failed to resize window:', e)
      }
    },
    { immediate: true },
  )
}
