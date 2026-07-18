<template>
  <div class="footer">
    <div class="footer-left">
      <span v-if="(sessionMode === 'inline_plugin' || sessionMode === 'full_page_plugin') && panelType">{{ panelType }}</span>
      <span v-else-if="resultCount > 0">{{ resultCount }} 个结果</span>
      <span v-else>就绪</span>
    </div>
    <div class="footer-actions" v-if="actions && actions.length > 0">
      <button
        v-for="(action, i) in actions"
        :key="action.id"
        class="action-btn"
        :class="{ 'is-default': action.isDefault, 'is-selected': i === selectedActionIndex }"
        @click="$emit('action-execute', action.id)"
      >
        {{ action.label }}
        <kbd v-if="action.shortcutKey" class="shortcut">{{ action.shortcutKey }}</kbd>
      </button>
    </div>
    <div class="footer-right">
      <n-button text size="small" @click="openSettingsWindow">
        <template #icon>
        <n-icon :size="14">
          <Settings />
        </n-icon>
        </template>
        设置
      </n-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { NButton, NIcon } from 'naive-ui'
import { Settings } from 'lucide-vue-next'
import { useSettings } from '../../composables/useSettings'
import type { SessionMode } from '../../stores/search-store'
import type { ResultAction } from '../../bridge/contract'

defineProps<{
  resultCount: number
  sessionMode: SessionMode
  panelType: string | null
  actions: ResultAction[]
  selectedActionIndex: number
}>()

defineEmits<{
  (e: 'action-execute', actionId: string): void
}>()

const { openSettings } = useSettings()
const openSettingsWindow = () => openSettings()
</script>

<style scoped>
.footer {
  height: var(--footer-height);
  flex-shrink: 0;
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 16px 10px; /* Increased padding */
  /* Remove hard dividing line */
  border-top: 1px solid transparent; 
  font-size: var(--font-size-sm);
  font-family: var(--footer-font-family);
  color: var(--text-secondary);
  gap: 12px;
  background: var(--bg-primary); /* Ensure background is solid */
  position: relative;
  z-index: 10;
  box-shadow: var(--shadow-footer);
}

.footer-left, .footer-right {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
  opacity: 0.8;
}

.footer-actions {
  display: flex;
  gap: 6px;
  align-items: center;
  flex-shrink: 1;
  overflow: hidden;
  margin-left: auto;
}

.action-btn {
  padding: 4px 10px;
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
  background: var(--bg-secondary);
  border: none;
  border-radius: 6px;
  cursor: pointer;
  font-family: inherit;
  white-space: nowrap;
  transition: all 0.2s ease;
}


.action-btn.is-selected {
  color: var(--accent-color);
  background: var(--primary-color-alpha); /* Subtle highlight */
}

.action-btn:hover {
  background: var(--hover-color);
}

.shortcut {
  margin-left: 6px;
  opacity: 0.5;
  font-size: 11px;
  font-family: monospace;
}
</style>
