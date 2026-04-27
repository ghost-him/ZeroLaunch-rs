<template>
  <div class="context-menu" :style="positionStyle" v-if="visible">
    <div
      v-for="item in items"
      :key="item.key"
      class="ctx-item"
      @click="onClick(item)"
    >
      {{ item.label }}
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

interface CtxItem {
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

const positionStyle = computed(() => ({
  left: props.x + 'px',
  top: props.y + 'px',
}))

function onClick(item: CtxItem) {
  item.action?.()
  emit('close')
}
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
