import { watch, onMounted, onUnmounted, nextTick } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { PhysicalSize } from '@tauri-apps/api/dpi'
import { useSearchStore } from '../stores/search-store'

function readCssProp(name: string): number {
  return parseFloat(getComputedStyle(document.documentElement).getPropertyValue(name).trim()) || 0
}

/// 通知浮层底边与窗口底边保留的间距（与通知层顶部 12px 留白对称）
const NOTIFICATION_BOTTOM_GAP = 12

export function useWindowResize() {
  const store = useSearchStore()
  const appWindow = getCurrentWindow()
  let observer: ResizeObserver | null = null
  /// 最近一次已下发的目标尺寸（物理像素）：目标未变则不重复下发，
  /// 避免自身 setSize 触发的 resize 事件形成回调循环；下发失败时清空，供下次触发重试。
  let appliedSize = { width: 0, height: 0 }

  /// 通知浮层的观察者：容器随首个通知出现、随最后一个通知消失（Teleport 到 body），
  /// 容器自身高度等于视口高度，只能监听内部通知的增删来重新测算窗口尺寸。
  let layerObserver: MutationObserver | null = null
  let observedLayer: HTMLElement | null = null
  let bodyObserver: MutationObserver | null = null

  /** 同步通知浮层观察者：容器出现时监听其内部通知增删，容器消失时解除观察并回收窗口高度。 */
  function syncNotificationLayer() {
    const layer = document.querySelector<HTMLElement>('.n-notification-container')
    if (layer === observedLayer) return
    layerObserver?.disconnect()
    observedLayer = layer
    layerObserver = null
    if (layer) {
      layerObserver = new MutationObserver(() => {
        void resizeWindow()
      })
      layerObserver.observe(layer, { childList: true, subtree: true })
    }
    void resizeWindow()
  }

  /** 测算窗口应有的物理尺寸。
   * 宽度取 --window-width（CSS px），高度取 .window-frame 的当前渲染高度，
   * 两者都按 devicePixelRatio（＝系统缩放 × 页面缩放）折算为物理像素。
   * 不能只按窗口 scale_factor 换算（原来的 LogicalSize 做法）：页面按 devicePixelRatio
   * 渲染，两者不一致时（窗口跨不同 DPI 显示器后 scale_factor 未同步、页面被缩放）
   * 窗口比内容渲染尺寸小，面板与底栏底部被窗口边缘裁掉。
   * 通知浮层经 Teleport 挂到 body 且 position: fixed，不参与 .window-frame 布局，
   * 故高度还要容纳所有通知的下边缘，否则仅搜索栏高的窗口会把通知底部裁掉。
   * @returns 目标尺寸；.window-frame 尚未挂载时返回 null */
  function measureWindowSize(): { width: number; height: number } | null {
    const frame = document.querySelector('.window-frame')
    if (!frame) return null
    let notificationBottom = 0
    for (const el of document.querySelectorAll('.n-notification')) {
      notificationBottom = Math.max(notificationBottom, el.getBoundingClientRect().bottom)
    }
    const scale = window.devicePixelRatio || 1
    return {
      width: Math.round((readCssProp('--window-width') || 600) * scale),
      height: Math.round(
        Math.max(frame.getBoundingClientRect().height, notificationBottom + NOTIFICATION_BOTTOM_GAP) *
          scale,
      ),
    }
  }

  /** 按内容渲染尺寸同步窗口大小。
   * 幂等，可被任意事件重复触发；下发失败时清空记录，供下次触发重试。 */
  async function resizeWindow() {
    const size = measureWindowSize()
    if (!size) return
    if (size.width === appliedSize.width && size.height === appliedSize.height) return
    appliedSize = size

    try {
      // 以物理像素下发：LogicalSize 会被 Tauri 再按窗口 scale_factor 换算一次，
      // 与页面实际渲染比例不一致时会得到错误尺寸。
      await appWindow.setSize(new PhysicalSize(size.width, size.height))
    } catch (e) {
      appliedSize = { width: 0, height: 0 }
      console.warn('[useWindowResize] Failed to resize window:', e)
    }
  }

  // 当内容产生变化时，等待 DOM 渲染后测算尺寸。
  // 使用数组形式分别监听每个依赖，避免每次创建新对象导致 watch 无条件触发。
  watch(
    [
      () => store.sessionMode,
      () => store.isIdle,
      () => store.results.length,
    ],
    async () => {
      await nextTick()
      await resizeWindow()
    },
    { immediate: true, flush: 'post' }
  )

  /// 视口变化（跨 DPI 显示器移动、页面缩放等）后重新测算：这类变化不改变内容 CSS 高度，
  /// ResizeObserver 不会触发，必须单独监听；devicePixelRatio 可能晚于 resize 事件才更新，
  /// 故合并到帧上并多测几帧，直到目标尺寸稳定（测算幂等，重复代价可忽略）。
  let resizeRaf = 0
  let settleFrames = 0
  function onViewportResize() {
    settleFrames = 2
    if (!resizeRaf) resizeRaf = requestAnimationFrame(checkViewportSize)
  }
  function checkViewportSize() {
    resizeRaf = 0
    void resizeWindow()
    if (settleFrames-- > 0) resizeRaf = requestAnimationFrame(checkViewportSize)
  }

  onMounted(() => {
    const frame = document.querySelector('.window-frame')
    if (!frame) return

    // 监听可能由其他途径引起的高度变化（如插件面板异步加载数据、字体行高变化等）
    observer = new ResizeObserver(() => {
      void resizeWindow()
    })
    observer.observe(frame, { box: 'border-box' })
    window.addEventListener('resize', onViewportResize)

    // 通知容器是 Teleport 到 body 的直接子节点：监听 body 子节点增删即可捕获容器挂载/移除
    bodyObserver = new MutationObserver(syncNotificationLayer)
    bodyObserver.observe(document.body, { childList: true })
    syncNotificationLayer()
  })

  onUnmounted(() => {
    if (observer) {
      observer.disconnect()
    }
    bodyObserver?.disconnect()
    layerObserver?.disconnect()
    window.removeEventListener('resize', onViewportResize)
    if (resizeRaf) cancelAnimationFrame(resizeRaf)
  })

  return { resizeWindow }
}
