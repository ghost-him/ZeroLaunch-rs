import type { useSearchStore } from '@/stores/search-store'

export function handleParamPanelKey(e: KeyboardEvent, store: ReturnType<typeof useSearchStore>) {
  switch (e.key) {
    case 'Enter':
      e.preventDefault()
      store.confirmParamPanel()
      break
    case 'Escape':
      e.preventDefault()
      store.exitParamPanelMode()
      break
    case 'Tab':
      e.preventDefault()
      if (e.shiftKey) {
        store.paramPanelFocusPrev()
      } else {
        store.paramPanelFocusNext()
      }
      break
  }
}
