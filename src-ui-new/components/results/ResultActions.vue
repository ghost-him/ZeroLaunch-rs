<template>
  <div class="result-actions">
    <span
      v-for="(action, i) in actions"
      :key="action.id"
      class="action-btn"
      :class="{ 'is-default': action.isDefault, 'is-selected': i === selectedIndex }"
    >
      {{ action.label }}
      <span v-if="action.shortcutKey" class="shortcut">{{ action.shortcutKey }}</span>
    </span>
  </div>
</template>

<script setup lang="ts">
import type { ResultAction } from '../../bridge/contract'

defineProps<{
  actions: ResultAction[]
  selectedIndex: number
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
  border-radius: 4px;
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
