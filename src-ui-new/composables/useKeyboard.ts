import { onMounted, onUnmounted } from 'vue'
import { useSearchStore } from '../stores/search-store'

export function useKeyboard() {
  const store = useSearchStore()

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
      case 'Escape':
        e.preventDefault()
        if (store.query !== '') {
          store.query = ''
          store.results = []
          store.sessionMode = 'none'
        }
        break
      case 'Tab':
        e.preventDefault()
        // TODO: cycle through actions
        break
      case 'Home':
        e.preventDefault()
        store.selectedIndex = 0
        break
      case 'End':
        e.preventDefault()
        store.selectedIndex = Math.max(store.results.length - 1, 0)
        break
    }

    // Ctrl+数字: 快速触发
    if (ctrl && e.code.startsWith('Digit')) {
      const num = parseInt(e.code.replace('Digit', ''))
      if (num >= 1 && num <= 9) {
        e.preventDefault()
        store.doConfirm(num - 1)
      }
    }
  }

  onMounted(() => window.addEventListener('keydown', handleKeydown))
  onUnmounted(() => window.removeEventListener('keydown', handleKeydown))
}
