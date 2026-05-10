<template>
  <component
    :is="customRenderer"
    v-if="customRenderer"
    :item="item"
    :selected="selected"
    :index="index"
  />
  <div
    v-else
    class="result-item"
    :class="{ selected }"
    @click="$emit('click')"
    @dblclick="$emit('dblclick')"
    @contextmenu.prevent="onContextMenu"
  >
    <div class="item-icon">
      <IconDisplay :src="item.icon" :size="24" />
    </div>
    <div class="item-text">
      <div class="item-title">{{ item.title }}</div>
      <div class="item-subtitle" v-if="item.subtitle">{{ item.subtitle }}</div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import type { Component } from 'vue'
import type { ListItem } from '../../bridge/contract'
import { usePluginStore } from '../../stores/plugin-store'
import IconDisplay from '../common/IconDisplay.vue'
import type { CtxItem } from '../layout/ContextMenu.vue'

const props = defineProps<{
  item: ListItem
  selected: boolean
  index: number
}>()

const emit = defineEmits<{
  (e: 'click'): void
  (e: 'dblclick'): void
  (e: 'context-action', actionId: string): void
  (e: 'contextmenu', x: number, y: number, items: CtxItem[]): void
}>()

const pluginStore = usePluginStore()

const customRenderer = computed<Component | null>(() =>
  pluginStore.getResultItemComponent(props.item.targetType),
)

function onContextMenu(e: MouseEvent) {
  const items: CtxItem[] = props.item.actions.map((a) => ({
    key: a.id,
    label: a.label,
    action: () => emit('context-action', a.id),
  }))
  emit('contextmenu', e.clientX, e.clientY, items)
}
</script>

<style scoped>
.result-item {
  height: var(--result-item-height);
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 16px; /* Increased gap for breathing room */
  padding: 8px 16px; /* Larger padding */
  margin-bottom: 2px; /* Slight separation */
  border-radius: var(--radius-md); /* Softer corners */
  cursor: pointer;
  transition: all 0.2s ease; /* Smoother transition */
  border: 1px solid transparent; /* Prepare for possible active state without shifting layout */
}

.result-item:hover,
.result-item.selected {
  background-color: var(--hover-color); /* Use subtle hover color rather than strong secondary background */
}

.item-icon {
  flex-shrink: 0;
  width: 32px; /* Slightly larger icons */
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.item-text {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  justify-content: center;
}

.item-title {
  font-size: var(--font-size-lg);
  font-family: var(--result-item-font-family);
  font-weight: 500;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  letter-spacing: 0.2px;
}

.item-subtitle {
  font-size: var(--font-size-md);
  font-family: var(--result-item-font-family);
  color: var(--text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  margin-top: 2px;
}
</style>
