<template>
  <div class="result-list" data-no-drag>
    <ResultItem
      v-for="(item, index) in results"
      :key="item.id"
      :item="item"
      :selected="index === selectedIndex"
      :index="index"
      @click="onItemClick(index)"
      @dblclick="onItemDblClick(index)"
      @context-action="(actionId: string) => emit('context-action', index, actionId)"
      @contextmenu="(x: number, y: number, items: CtxItem[]) => emit('contextmenu', x, y, items)"
    />
    <div v-if="results.length === 0" class="no-results">
      无结果
    </div>
  </div>
</template>

<script setup lang="ts">
import ResultItem from './ResultItem.vue'
import type { ListItem } from '../../bridge/contract'
import type { CtxItem } from '../layout/ContextMenu.vue'

defineProps<{
  results: ListItem[]
  selectedIndex: number
}>()

const emit = defineEmits<{
  (e: 'select', index: number): void
  (e: 'confirm', index: number, actionIdx?: number): void
  (e: 'context-action', index: number, actionId: string): void
  (e: 'contextmenu', x: number, y: number, items: CtxItem[]): void
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
  --rl-padding-y: 8px;
  flex: 1;
  min-height: calc((var(--result-item-height) + 2px) * 1 + 2 * var(--rl-padding-y));
  max-height: calc((var(--result-item-height) + 2px) * var(--max-visible-results) + 2 * var(--rl-padding-y));
  overflow-y: auto;
  padding: var(--rl-padding-y) 12px;
}

/* Hide scrollbar for a cleaner look, similar to modern OS / sofast */
.result-list::-webkit-scrollbar {
  width: 4px;
}
.result-list::-webkit-scrollbar-thumb {
  background: rgba(0, 0, 0, 0.1);
  border-radius: 4px;
}
html.dark .result-list::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.1);
}

.no-results {
  padding: 32px 24px; /* More padding for empty state */
  text-align: center;
  color: var(--text-secondary);
  font-size: var(--font-size-base);
  opacity: 0.8;
}
</style>
