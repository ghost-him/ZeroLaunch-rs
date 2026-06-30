import type { useSearchStore } from '@/stores/search-store'

export function handleInlineParamKey(e: KeyboardEvent, store: ReturnType<typeof useSearchStore>) {
  switch (e.key) {
    case 'Enter':
      e.preventDefault()
      store.confirmInlineParam()
      break
    case 'Escape':
      e.preventDefault()
      store.exitInlineParamMode()
      break
    case 'Backspace':
      // 如果参数输入为空，退出行内模式
      if (store.inlineParamState?.paramInput === '') {
        e.preventDefault()
        store.exitInlineParamMode()
      }
      // 否则正常删除字符（不阻止默认行为）
      break
  }
}
