import { getCurrentWindow } from '@tauri-apps/api/window'
import type { useSearchStore } from '@/stores/search-store'

interface SearchKeyOptions {
  spaceIsEnter: boolean
  isEscHideWindowPriority: boolean
}

export function handleSearchModeKey(e: KeyboardEvent, store: ReturnType<typeof useSearchStore>, opts: SearchKeyOptions) {
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
      store.handleEnterInSearchMode()
      break
    case ' ':
      if (opts.spaceIsEnter) {
        e.preventDefault()
        store.doConfirm()
      } else if (store.tryEnterInlineParamMode()) {
        e.preventDefault()
      }
      break
    case 'Escape':
      e.preventDefault()
      if (opts.isEscHideWindowPriority) {
        getCurrentWindow().hide()
      } else if (store.query !== '') {
        store.query = ''
        store.results = []
        store.sessionMode = 'none'
      } else {
        getCurrentWindow().hide()
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
