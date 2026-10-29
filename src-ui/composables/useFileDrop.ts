// 设置窗文件拖放路由 —— 单监听 + 字段级投放目标注册表。
//
// 原生拖放开启后 webview 不会投递 HTML5 drop 事件，路径只能从 webview 级
// `onDragDropEvent` 取得；该事件只给物理像素坐标，所以按坐标命中已注册的元素，
// 取嵌套最深者（字段内部的投放目标优先于外层卡片）。
// 仅设置窗使用：`main.ts` 不挂载本路由。

import { computed, onBeforeUnmount, onMounted, ref, type ComputedRef, type Ref } from 'vue'
import { onFileDragDrop } from '../bridge/events'
import { pathKind } from '../bridge/commands'

/** 拖入项类型：`unavailable` 表示路径不存在、无权限或宿主判定失败。 */
export type DroppedPathKind = 'file' | 'directory' | 'unavailable'

/** 单个拖入路径及其类型。 */
export interface DroppedPath {
  path: string
  kind: DroppedPathKind
}

/** 投放目标作用域：field 按坐标命中，page 仅在该页无字段命中时整页兜底。 */
export type DropScope = 'field' | 'page'

/** 注册表条目。 */
interface DropTarget {
  /** 目标标识，用于悬停高亮比对。 */
  id: symbol
  /** 参与命中判定的元素。 */
  el: HTMLElement
  /** 命中优先级作用域。 */
  scope: DropScope
  /** 投放回调，入参已带宿主判定出的类型。 */
  onDrop: (paths: DroppedPath[]) => void
}

/** 已注册目标集合（模块级单例：整个设置窗共享一份）。 */
const targets = new Set<DropTarget>()
/** 当前拖拽悬停的目标；null 表示未命中任何目标。 */
const hoveredId = ref<symbol | null>(null)

/** 物理像素坐标 → CSS 像素坐标（`getBoundingClientRect` 所在坐标系）。 */
function toCssPoint(position: { x: number; y: number }): { x: number; y: number } {
  const ratio = window.devicePixelRatio || 1
  return { x: position.x / ratio, y: position.y / ratio }
}

/** 元素在 DOM 中的嵌套深度，用于重叠目标取最深者。 */
function depthOf(el: HTMLElement): number {
  let depth = 0
  let node: HTMLElement | null = el
  while (node) {
    depth += 1
    node = node.parentElement
  }
  return depth
}

/** 按坐标解析投放目标：字段目标取最深命中者，否则回落到页面级目标。 */
function resolveTarget(point: { x: number; y: number }): DropTarget | null {
  let matched: DropTarget | null = null
  let matchedDepth = -1
  for (const target of targets) {
    if (target.scope !== 'field') continue
    const rect = target.el.getBoundingClientRect()
    if (point.x < rect.left || point.x > rect.right) continue
    if (point.y < rect.top || point.y > rect.bottom) continue
    const depth = depthOf(target.el)
    if (depth > matchedDepth) {
      matched = target
      matchedDepth = depth
    }
  }
  if (matched) return matched
  for (const target of targets) {
    if (target.scope === 'page') return target
  }
  return null
}

/** 把原始路径批量交给宿主判定类型；IPC 失败时降级为 unavailable，由投放方提示。 */
async function resolvePaths(paths: string[]): Promise<DroppedPath[]> {
  try {
    return await pathKind(paths)
  } catch (error) {
    console.warn('[settings] 拖入路径类型判定失败:', error)
    return paths.map((path) => ({ path, kind: 'unavailable' as const }))
  }
}

/**
 * 在设置窗根部挂载唯一的 webview 拖放监听；卸载时解除。
 * 参数：无。
 * 返回：无。
 */
export function useFileDropRouter(): void {
  let unlisten: (() => void) | null = null
  onMounted(async () => {
    try {
      unlisten = await onFileDragDrop(async (payload) => {
        if (payload.type === 'leave') {
          hoveredId.value = null
          return
        }
        const point = toCssPoint(payload.position)
        if (payload.type === 'enter' || payload.type === 'over') {
          hoveredId.value = resolveTarget(point)?.id ?? null
          return
        }
        const target = resolveTarget(point)
        hoveredId.value = null
        if (!target) return
        target.onDrop(await resolvePaths(payload.paths))
      })
    } catch (error) {
      // 监听失败只降级拖放能力，不影响设置页其余交互
      console.warn('[settings] 拖放监听注册失败:', error)
    }
  })
  onBeforeUnmount(() => {
    unlisten?.()
    unlisten = null
  })
}

/**
 * 注册一个投放目标。
 * 参数：`el` — 目标元素引用；`options.scope` — 命中作用域（默认 field）；
 * `options.enabled` — false 时跳过注册（只读字段等）；`options.onDrop` — 投放回调。
 * 返回：`isHovered` — 拖拽悬停在该目标上的响应式标记，供高亮渲染。
 */
export function useFileDropTarget(
  el: Ref<HTMLElement | null>,
  options: { scope?: DropScope; enabled?: boolean; onDrop: (paths: DroppedPath[]) => void },
): { isHovered: ComputedRef<boolean> } {
  const id = Symbol('file-drop-target')
  let registered: DropTarget | null = null

  onMounted(() => {
    const node = el.value
    if (!node || options.enabled === false) return
    registered = { id, el: node, scope: options.scope ?? 'field', onDrop: options.onDrop }
    targets.add(registered)
  })

  onBeforeUnmount(() => {
    if (registered) targets.delete(registered)
    if (hoveredId.value === id) hoveredId.value = null
  })

  return { isHovered: computed(() => hoveredId.value === id) }
}
