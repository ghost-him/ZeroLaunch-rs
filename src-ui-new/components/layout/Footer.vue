<template>
  <div class="footer">
    <div class="footer-left">
      <span v-if="sessionMode === 'plugin' && panelType">{{ panelType }}</span>
      <span v-else-if="resultCount > 0">{{ resultCount }} 个结果</span>
      <span v-else>就绪</span>
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

defineProps<{
  resultCount: number
  sessionMode: SessionMode
  panelType: string | null
}>()

const { openSettings } = useSettings()
const openSettingsWindow = () => openSettings()
</script>

<style scoped>
.footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 4px 16px 8px;
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
}

.footer-left, .footer-right {
  display: flex;
  align-items: center;
  gap: 8px;
}
</style>
