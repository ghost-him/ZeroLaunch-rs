<template>
  <div class="footer">
    <div class="footer-left">
      <span v-if="sessionMode === 'plugin' && panelType">{{ panelType }}</span>
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
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" />
              <circle cx="12" cy="12" r="3" />
            </svg>
          </n-icon>
        </template>
        设置
      </n-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { NButton, NIcon } from 'naive-ui'
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
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 4px 12px 6px;
  border-top: 1px solid var(--border-color);
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
  gap: 8px;
}

.footer-left, .footer-right {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.footer-actions {
  display: flex;
  gap: 4px;
  align-items: center;
  flex-shrink: 1;
  overflow: hidden;
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
  white-space: nowrap;
}

.action-btn.is-default {
  color: var(--accent-color);
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
