<template>
  <div class="footer">
    <div class="footer-left">
      <span
        v-if="(sessionMode === 'plugin_panel' || sessionMode === 'plugin_immersive') && currentPluginMeta"
        class="footer-plugin-id"
      >
        <img v-if="currentPluginMeta.mode === 'panel' && currentPluginMeta.icon" :src="currentPluginMeta.icon" class="footer-plugin-icon" alt="" />
        <span>{{ resolveText(currentPluginMeta.name) }}</span>
      </span>
      <span v-else-if="resultCount > 0">{{ t('search.candidates', { count: resultCount }) }}</span>
      <span v-else>{{ t('common.ready') }}</span>
    </div>
    <div class="footer-actions" v-if="actions && actions.length > 0">
      <button
        v-for="(action, i) in actions"
        :key="action.id"
        class="action-btn"
        :class="{ 'is-default': action.isDefault, 'is-selected': i === selectedActionIndex }"
        @click="$emit('action-execute', action.id)"
      >
        {{ resolveText(action.label) }}
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
        {{ t('footer.settings') }}
      </n-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { NButton, NIcon } from 'naive-ui'
import { Settings } from 'lucide-vue-next'
import { useI18n } from 'vue-i18n'
import { resolveText } from '../../i18n'
import { useSettings } from '../../composables/useSettings'
import { useSearchStore, type SessionMode } from '../../stores/search-store'
import type { ResultAction } from '../../bridge/contract'

const props = defineProps<{
  resultCount: number
  sessionMode: SessionMode
  actions: ResultAction[]
  selectedActionIndex: number
}>()

defineEmits<{
  (e: 'action-execute', actionId: string): void
}>()

const { openSettings } = useSettings()
const openSettingsWindow = () => openSettings()
const { t } = useI18n()

const searchStore = useSearchStore()

/// 当前插件面板的元数据（名称/图标）：图标仅 panel 形态插件展示，行内插件不展示。
const currentPluginMeta = computed(() => {
  const id = searchStore.currentPluginId
  if (!id) return null
  return searchStore.pluginMeta[id] ?? null
})
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

.footer-plugin-id {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  white-space: nowrap;
}

.footer-plugin-icon {
  width: 16px;
  height: 16px;
  border-radius: 4px;
  object-fit: contain;
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
