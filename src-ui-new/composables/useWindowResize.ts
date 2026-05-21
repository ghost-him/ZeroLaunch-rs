import { watch, onMounted, onUnmounted, nextTick } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { LogicalSize } from '@tauri-apps/api/dpi'
import { useSearchStore } from '../stores/search-store'

function readCssProp(name: string): number {
  return parseFloat(getComputedStyle(document.documentElement).getPropertyValue(name).trim()) || 0
}

export function useWindowResize() {
  const store = useSearchStore()
  const appWindow = getCurrentWindow()
  let observer: ResizeObserver | null = null
  let lastHeight = 0

  /** 根据当前 DOM 布局重新计算并设置窗口尺寸 */
  async function resizeWindow() {
    const frame = document.querySelector('.window-frame')
    if (!frame) return

    const rect = frame.getBoundingClientRect()
    const windowWidth = readCssProp('--window-width') || 600

    if (Math.abs(rect.height - lastHeight) < 0.5) return
    lastHeight = rect.height

    try {
      await appWindow.setSize(new LogicalSize(windowWidth, rect.height))
    } catch (e) {
      console.warn('[useWindowResize] Failed to resize window:', e)
    }
  }

  // 当内容产生变化时，等待 DOM 渲染后测算尺寸
  watch(
    () => ({
      mode: store.sessionMode,
      isIdle: store.isIdle,
      resultCount: store.results.length,
      keepSearchBar: store.keepSearchBar,
    }),
    async () => {
      await nextTick()
      await resizeWindow()
    },
    { immediate: true, flush: 'post' }
  )

  onMounted(() => {
    const frame = document.querySelector('.window-frame')
    if (!frame) return

    // 监听可能由其他途径引起的高度变化（如插件面板异步加载数据等）
    observer = new ResizeObserver(async (entries) => {
      const height =
        entries[entries.length - 1].borderBoxSize?.[0]?.blockSize ??
        entries[entries.length - 1].contentRect.height + readCssProp('--window-border-width') * 2
      const windowWidth = readCssProp('--window-width') || 600

      if (Math.abs(height - lastHeight) < 0.5) return
      lastHeight = height

      try {
        await appWindow.setSize(new LogicalSize(windowWidth, height))
      } catch (e) {
        console.warn('[useWindowResize] Failed to resize from observer:', e)
      }
    })

    observer.observe(frame, { box: 'border-box' })
  })

  onUnmounted(() => {
    if (observer) {
      observer.disconnect()
    }
  })

  return { resizeWindow }
}
