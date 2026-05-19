import { onMounted, onUnmounted } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useSearchStore } from '../stores/search-store'
import { useConfigStore } from '../stores/config-store'

export function useKeyboard() {
  const store = useSearchStore()
  const configStore = useConfigStore()

  function handleKeydown(e: KeyboardEvent) {
    // 沉浸态：不处理键盘
    if (store.sessionMode === 'plugin' && !store.keepSearchBar) return

    const ctrl = e.ctrlKey || e.metaKey

    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault()
        store.selectNext()
        break
      case 'ArrowUp':
        e.preventDefault()
        store.selectPrev()
        break
      case 'Enter':
        e.preventDefault()
        store.doConfirm()
        break
      case ' ':
        {
          const wb = configStore.settings['window-behavior'] as Record<string, boolean> | undefined
          if (wb?.space_is_enter) {
            e.preventDefault()
            store.doConfirm()
          }
        }
        break
      case 'Escape':
        e.preventDefault()
        {
          const wb = configStore.settings['window-behavior'] as Record<string, boolean> | undefined
          if (wb?.is_esc_hide_window_priority) {
            getCurrentWindow().hide()
          } else if (store.query !== '') {
            store.query = ''
            store.results = []
            store.sessionMode = 'none'
          } else {
            getCurrentWindow().hide()
          }
        }
        break
      case 'Tab': {
        e.preventDefault()
        const item = store.selectedItem
        if (item && item.actions.length > 0) {
          store.selectedActionIndex =
            (store.selectedActionIndex + 1) % item.actions.length
        }
        break
      }
      case 'Home':
        e.preventDefault()
        store.selectedIndex = 0
        break
      case 'End':
        e.preventDefault()
        store.selectedIndex = Math.max(store.results.length - 1, 0)
        break
    }

    // Ctrl+数字: 触发对应 action
    if (ctrl && e.code.startsWith('Digit')) {
      const num = parseInt(e.code.replace('Digit', ''))
      if (num >= 1 && num <= 9) {
        e.preventDefault()
        const actionIdx = num - 1
        const item = store.selectedItem
        if (item && actionIdx < item.actions.length) {
          store.doConfirm(undefined, item.actions[actionIdx].id)
        }
      }
    }
  }

  onMounted(() => window.addEventListener('keydown', handleKeydown))
  onUnmounted(() => window.removeEventListener('keydown', handleKeydown))
}
