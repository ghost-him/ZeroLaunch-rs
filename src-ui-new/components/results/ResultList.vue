<template>
  <div class="result-list" :style="{ maxHeight: maxHeight + 'px' }">
    <ResultItem
      v-for="(item, index) in results"
      :key="item.id"
      :item="item"
      :selected="index === selectedIndex"
      :index="index"
      @click="onItemClick(index)"
      @dblclick="onItemDblClick(index)"
      @context-action="(actionId: string) => emit('context-action', index, actionId)"
    />
    <div v-if="results.length === 0" class="no-results">
      无结果
    </div>
  </div>
</template>

<script setup lang="ts">
import ResultItem from './ResultItem.vue'
import type { ListItem } from '../../bridge/contract'

defineProps<{
  results: ListItem[]
  selectedIndex: number
  maxHeight?: number
}>()

const emit = defineEmits<{
  (e: 'select', index: number): void
  (e: 'confirm', index: number, actionIdx?: number): void
  (e: 'context-action', index: number, actionId: string): void
}>()

function onItemClick(index: number) {
  emit('select', index)
}

function onItemDblClick(index: number) {
  emit('confirm', index)
}
</script>

<style scoped>
.result-list {
  overflow-y: auto;
  padding: 0 8px;
}

.no-results {
  padding: 24px;
  text-align: center;
  color: var(--text-secondary);
  font-size: var(--font-size-sm);
}
</style>
