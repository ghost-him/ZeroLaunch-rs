<template>
  <Teleport to="body">
    <div v-if="visible" ref="menuRef" class="context-menu" :style="positionStyle" @contextmenu.prevent>
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
import { computed, ref, watch, onUnmounted } from 'vue'

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

const positionStyle = computed(() => ({
  left: props.x + 'px',
  top: props.y + 'px',
}))

function onClick(item: CtxItem) {
  item.action?.()
  emit('close')
}

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
