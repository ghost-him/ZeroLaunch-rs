<template>
  <div class="settings-view">
    <div class="settings-header">
      <h2>设置</h2>
      <n-button text @click="closeWindow">
        <n-icon :size="18">
          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <line x1="18" y1="6" x2="6" y2="18" />
            <line x1="6" y1="6" x2="18" y2="18" />
          </svg>
        </n-icon>
      </n-button>
    </div>

    <div class="settings-body">
      <SettingsSidebar
        v-if="!loading"
        :sidebar-items="sidebarItems"
        :selected-id="selectedId"
        @select="onSelectItem"
        @toggle="onToggleComponent"
      />

      <div class="settings-content">
        <div v-if="loading" class="loading-state">
          <n-spin :size="24" />
        </div>
        <div v-else-if="loadErr" class="error-state">
          <n-text type="error">{{ loadErr }}</n-text>
          <n-button size="small" @click="init">重试</n-button>
        </div>
        <DynamicForm
          v-else-if="selectedId && selectedSchema && selectedSettings"
          :key="selectedId"
          :schema="selectedSchema"
          :current-settings="selectedSettings"
          @reload="onReloadComponent"
        />
        <div v-else-if="selectedId === 'appearance'" class="static-panel">
          <h3>外观</h3>
          <div class="static-row">
            <span>主题</span>
            <n-button-group>
              <n-button :type="!themeStore.isDark ? 'primary' : 'default'" size="small" @click="themeStore.setTheme(false)">浅色</n-button>
              <n-button :type="themeStore.isDark ? 'primary' : 'default'" size="small" @click="themeStore.setTheme(true)">深色</n-button>
            </n-button-group>
          </div>
        </div>
        <div v-else-if="selectedId === 'about'" class="static-panel">
          <h3>关于</h3>
          <div class="static-row">
            <span>ZeroLaunch-rs</span>
            <n-text depth="3">v0.6.12</n-text>
          </div>
        </div>
        <div v-else class="empty-hint">
          <n-text depth="3">从左侧选择一个组件</n-text>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { NButton, NIcon, NSpin, NText, NButtonGroup } from 'naive-ui'
import { getCurrentWindow } from '@tauri-apps/api/window'
import SettingsSidebar from '../components/settings/SettingsSidebar.vue'
import DynamicForm from '../components/settings/DynamicForm.vue'
import { useConfigStore } from '../stores/config-store'
import { useThemeStore } from '../stores/theme-store'
import { useSettings } from '../composables/useSettings'
import { onConfigChanged } from '../bridge/events'
import type { ComponentInfo, ComponentSchema } from '../bridge/contract'

const configStore = useConfigStore()
const themeStore = useThemeStore()
const { buildSidebarItems } = useSettings()

const loading = ref(true)
const loadErr = ref<string | null>(null)
const selectedId = ref<string | null>(null)
const selectedSchema = ref<ComponentSchema | null>(null)
const selectedSettings = ref<Record<string, unknown> | null>(null)

let unlistenConfig: (() => void) | null = null

function getComponentsList(): ComponentInfo[] {
  return Object.values(configStore.components)
}

const sidebarItems = computed(() => buildSidebarItems(getComponentsList()))

async function init() {
  loading.value = true
  loadErr.value = null
  try {
    await configStore.loadAllComponents()
  } catch (e) {
    loadErr.value = String(e)
  } finally {
    loading.value = false
  }
}

async function onSelectItem(itemId: string) {
  selectedId.value = itemId

  // Static sidebar items
  if (itemId === 'appearance' || itemId === 'about') {
    selectedSchema.value = null
    selectedSettings.value = null
    return
  }

  try {
    const [schema, settings] = await Promise.all([
      configStore.getSchema(itemId),
      configStore.getSettings(itemId),
    ])
    selectedSchema.value = schema
    selectedSettings.value = settings as Record<string, unknown>
  } catch (e) {
    loadErr.value = String(e)
  }
}

async function onToggleComponent(componentId: string, enabled: boolean) {
  try {
    await configStore.setEnabled(componentId, enabled)
  } catch (e) {
    loadErr.value = String(e)
  }
}

async function onReloadComponent() {
  if (!selectedId.value) return
  try {
    const settings = await configStore.getSettings(selectedId.value)
    selectedSettings.value = settings as Record<string, unknown>
  } catch (e) {
    loadErr.value = String(e)
  }
}

function closeWindow() {
  getCurrentWindow().close()
}

onMounted(async () => {
  await init()

  unlistenConfig = await onConfigChanged((payload) => {
    // Cross-window sync: if the changed component is currently selected, reload
    if (payload.componentId === selectedId.value) {
      onReloadComponent()
    }
  })
})

onUnmounted(() => {
  unlistenConfig?.()
})
</script>

<style scoped>
.settings-view {
  height: 100vh;
  display: flex;
  flex-direction: column;
  background: var(--bg-primary);
  color: var(--text-primary);
}

.settings-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 20px;
  border-bottom: 1px solid var(--border-color);
}

.settings-header h2 {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
}

.settings-body {
  display: flex;
  flex: 1;
  overflow: hidden;
}

.settings-content {
  flex: 1;
  overflow-y: auto;
}

.loading-state,
.error-state,
.empty-hint {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  gap: 12px;
}

.static-panel {
  padding: 24px;
}

.static-panel h3 {
  margin: 0 0 16px;
  font-size: 16px;
  font-weight: 600;
}

.static-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 0;
}
</style>
