<template>
  <Teleport to="body">
    <div v-if="visible" ref="menuRef" class="context-menu" :style="menuStyle" @contextmenu.prevent>
      <div
        v-for="item in items"
        :key="item.key"
        class="ctx-item"
        @click="onClick(item)"
      >
        {{ item.label }}
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, watch, onUnmounted, nextTick } from 'vue'

export interface CtxItem {
  key: string
  label: string
  action?: () => void
}

const props = defineProps<{
  visible: boolean
  x: number
  y: number
  items: CtxItem[]
}>()

const emit = defineEmits<{
  (e: 'close'): void
}>()

const menuRef = ref<HTMLElement | null>(null)

/// 当前菜单位置样式 — 由 watch 根据窗口边界自动调整
const menuStyle = ref({ left: '0px', top: '0px' })

function onClick(item: CtxItem) {
  item.action?.()
  emit('close')
}

// ---- 智能定位：确保菜单完全在窗口可见区域内 ----

watch(
  () => [props.visible, props.x, props.y],
  async () => {
    if (!props.visible) {
      menuStyle.value = { left: '0px', top: '0px' }
      return
    }

    // 等待 DOM 渲染完成（浏览器尚未开始绘制）
    await nextTick()
    if (!menuRef.value) return

    const rect = menuRef.value.getBoundingClientRect()
    const vw = window.innerWidth
    const vh = window.innerHeight

    let left = props.x
    let top = props.y

    // 避免超出右边界
    if (left + rect.width > vw) {
      left = Math.max(vw - rect.width - 8, 0)
    }
    // 避免超出左边界
    if (left < 0) {
      left = 8
    }
    // 避免超出下边界
    if (top + rect.height > vh) {
      top = Math.max(vh - rect.height - 8, 0)
    }
    // 避免超出上边界
    if (top < 0) {
      top = 8
    }

    menuStyle.value = {
      left: left + 'px',
      top: top + 'px',
    }
  },
  { immediate: true },
)

// ---- 点击外部关闭（不使用 backdrop，避免抢夺焦点） ----

function onDocumentPointerDown(e: PointerEvent) {
  if (!menuRef.value) return
  if (menuRef.value.contains(e.target as Node)) return
  // 右键由 contextmenu 处理器负责
  if (e.button === 2) return
  emit('close')
}

function onDocumentContextMenu(e: MouseEvent) {
  // 如果事件已被其他组件处理（调用了 preventDefault），说明该组件会打开新菜单
  if (e.defaultPrevented) return
  if (!menuRef.value) return
  if (menuRef.value.contains(e.target as Node)) return
  // 右键点击在菜单外部（空白区域），关闭菜单
  emit('close')
}

function onDocumentKeyDown(e: KeyboardEvent) {
  if (e.key === 'Escape') {
    emit('close')
  }
}

watch(
  () => props.visible,
  (visible) => {
    if (visible) {
      document.addEventListener('pointerdown', onDocumentPointerDown, true)
      document.addEventListener('contextmenu', onDocumentContextMenu, true)
      document.addEventListener('keydown', onDocumentKeyDown, true)
    } else {
      document.removeEventListener('pointerdown', onDocumentPointerDown, true)
      document.removeEventListener('contextmenu', onDocumentContextMenu, true)
      document.removeEventListener('keydown', onDocumentKeyDown, true)
    }
  },
  { immediate: true },
)

onUnmounted(() => {
  document.removeEventListener('pointerdown', onDocumentPointerDown, true)
  document.removeEventListener('contextmenu', onDocumentContextMenu, true)
  document.removeEventListener('keydown', onDocumentKeyDown, true)
})
</script>

<style scoped>
.context-menu {
  position: fixed;
  z-index: 9999;
  min-width: 140px;
  padding: 4px 0;
  background: var(--bg-primary);
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  box-shadow: var(--shadow-md);
}

.ctx-item {
  padding: 6px 14px;
  font-size: var(--font-size-sm);
  cursor: pointer;
}

.ctx-item:hover {
  background: var(--bg-secondary);
}
</style>

