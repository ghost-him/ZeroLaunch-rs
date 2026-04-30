<template>
  <div class="result-actions">
    <button
      v-for="(action, i) in actions"
      :key="action.id"
      class="action-btn"
      :class="{ 'is-default': action.isDefault, 'is-selected': i === selectedIndex }"
      @click="$emit('execute', i)"
    >
      {{ action.label }}
      <kbd v-if="action.shortcutKey" class="shortcut">{{ action.shortcutKey }}</kbd>
    </button>
  </div>
</template>

<script setup lang="ts">
import type { ResultAction } from '../../bridge/contract'

defineProps<{
  actions: ResultAction[]
  selectedIndex: number
}>()

defineEmits<{
  (e: 'execute', index: number): void
}>()
</script>

<style scoped>
.result-actions {
  display: flex;
  gap: 4px;
  flex-shrink: 0;
}

.action-btn {
  padding: 2px 8px;
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
  background: var(--bg-secondary);
  border: none;
  border-radius: 4px;
  cursor: pointer;
  font-family: inherit;
}

.action-btn.is-selected {
  color: var(--accent-color);
  background: rgba(32, 128, 240, 0.1);
}

.shortcut {
  margin-left: 4px;
  opacity: 0.6;
  font-size: 10px;
}
</style>
