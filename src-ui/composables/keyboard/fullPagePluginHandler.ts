import type { useSearchStore } from '@/stores/search-store'

export function handleFullPagePluginKey(e: KeyboardEvent, store: ReturnType<typeof useSearchStore>) {
  e.preventDefault()

  // Escape 退出全页面模式
  if (e.key === 'Escape') {
    store.exitFullPagePlugin()
    return
  }

  // 其他按键由插件处理（未来通过 bridge_plugin_key_event 转发）
}
