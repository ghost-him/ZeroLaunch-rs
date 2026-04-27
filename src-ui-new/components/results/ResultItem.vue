<template>
  <div class="result-item" :class="{ selected }" @click="$emit('click')" @dblclick="$emit('dblclick')">
    <div class="item-icon">
      <IconDisplay :src="item.icon" :size="24" />
    </div>
    <div class="item-text">
      <div class="item-title">{{ item.title }}</div>
      <div class="item-subtitle" v-if="item.subtitle">{{ item.subtitle }}</div>
    </div>
    <ResultActions
      v-if="selected && item.actions.length > 0"
      :actions="item.actions"
      :selected-index="0"
    />
  </div>
</template>

<script setup lang="ts">
import type { ListItem } from '../../bridge/contract'
import IconDisplay from '../common/IconDisplay.vue'
import ResultActions from './ResultActions.vue'

defineProps<{
  item: ListItem
  selected: boolean
  index: number
}>()

defineEmits<{
  (e: 'click'): void
  (e: 'dblclick'): void
}>()
</script>

<style scoped>
.result-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px 8px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: background-color 0.1s;
}

.result-item:hover,
.result-item.selected {
  background-color: var(--bg-secondary);
}

.item-icon {
  flex-shrink: 0;
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.item-text {
  flex: 1;
  min-width: 0;
}

.item-title {
  font-size: var(--font-size-base);
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.item-subtitle {
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
