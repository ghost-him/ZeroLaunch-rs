import type { useSearchStore } from '@/stores/search-store'

export function handleInlinePluginKey(e: KeyboardEvent, store: ReturnType<typeof useSearchStore>) {
  // 行内插件保留搜索栏，所以大部分键给输入框处理
  switch (e.key) {
    case 'Escape':
      e.preventDefault()
      store.exitPluginMode()
      break
    case 'Enter':
      e.preventDefault()
      store.confirmPluginAction()
      break
  }
}
