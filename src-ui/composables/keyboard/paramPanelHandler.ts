import type { useSearchStore } from '@/stores/search-store'

export function handleParamPanelKey(e: KeyboardEvent, store: ReturnType<typeof useSearchStore>) {
  switch (e.key) {
    case 'Enter': {
      e.preventDefault()
      const state = store.paramPanelState
      if (state && state.focusedFieldIndex < state.fields.length - 1) {
        store.paramPanelFocusNext()
      } else {
        store.confirmParamPanel()
      }
      break
    }
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
