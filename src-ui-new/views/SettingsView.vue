<template>
  <div class="settings-view">
    <div class="settings-header">
      <h2>{{ $t('settings.title') }}</h2>
    </div>

    <div class="settings-body">
      <SettingsSidebar
        v-if="!loading"
        :sidebar-items="sidebarItems"
        :selected-id="selectedId"
        @select="onSelectItem"
      />

      <div class="settings-content">
        <div v-if="loading" class="loading-state">
          <n-spin :size="24" />
        </div>
        <div v-else-if="loadErr" class="error-state">
          <n-text type="error">{{ loadErr }}</n-text>
          <n-button size="small" @click="init">{{ $t('settings.saveFailed') }}</n-button>
        </div>
        
        <!-- Plugin Inspector -->
        <PluginInspector v-else-if="selectedId === 'category_inspector'" />

        <!-- About -->
        <div v-else-if="selectedId === 'category_about'" class="static-panel">
          <h3>{{ $t('settings.about') }}</h3>
          <div class="static-row">
            <span>ZeroLaunch-rs</span>
            <n-text depth="3">v0.6.12</n-text>
          </div>
        </div>

        <!-- Dynamic Category Views -->
        <template v-else-if="selectedCategory">
          <div class="category-header">
            <h3>{{ selectedCategory.label }}</h3>
          </div>
          
          <div class="category-scroll-area">
            <CategoryViewPipeline
              v-if="selectedCategory.type === 'pipeline' && selectedCategory.components"
              :components="selectedCategory.components"
            />
            <CategoryViewTabs
              v-else-if="selectedCategory.type === 'tabs' && selectedCategory.components"
              :components="selectedCategory.components"
            />
            <CategoryViewList
              v-else-if="selectedCategory.type === 'plugin' && selectedCategory.components"
              :components="selectedCategory.components"
              :show-toggle="true"
            />
            <CategoryViewList
              v-else-if="selectedCategory.components"
              :components="selectedCategory.components"
            />
            <div v-else class="empty-hint">
              <n-text depth="3">无可用设置</n-text>
            </div>
          </div>
        </template>

        <div v-else class="empty-hint">
          <n-text depth="3">从左侧选择一个组件</n-text>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { NButton, NSpin, NText, useNotification } from 'naive-ui'
import SettingsSidebar from '../components/settings/SettingsSidebar.vue'
import CategoryViewList from '../components/settings/CategoryViewList.vue'
import CategoryViewPipeline from '../components/settings/CategoryViewPipeline.vue'
import CategoryViewTabs from '../components/settings/CategoryViewTabs.vue'
import PluginInspector from './PluginInspector.vue'
import { useConfigStore } from '../stores/config-store'
import { buildSidebarItems } from '../utils/settingsSidebar'
import { registerErrorHandler } from '../bridge/commands'
import type { BridgeError } from '../bridge/commands'
import type { ComponentInfo } from '../bridge/contract'

const configStore = useConfigStore()
const notification = useNotification()

const loading = ref(true)
const loadErr = ref<string | null>(null)
const selectedId = ref<string | null>('category_core')

function getComponentsList(): ComponentInfo[] {
  return Object.values(configStore.components)
}

const sidebarItems = computed(() => buildSidebarItems(getComponentsList()))

const selectedCategory = computed(() => {
  if (!selectedId.value) return null
  
  // Find in top level
  for (const item of sidebarItems.value) {
    if (item.key === selectedId.value) return item
    
    // Find in nested plugins
    if (item.items) {
      const found = item.items.find(sub => sub.key === selectedId.value)
      if (found) return found
    }
  }
  return null
})

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

function onSelectItem(itemId: string) {
  selectedId.value = itemId
}

onMounted(async () => {
  registerErrorHandler((error: BridgeError) => {
    notification.error({
      title: error.code,
      content: error.message,
      duration: 5000,
    })
  })

  await init()
})
</script>

<style scoped>
.settings-view {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background-color: var(--bg-color);
  color: var(--text-color);
}

.settings-header {
  flex-shrink: 0;
  height: 48px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 16px;
  border-bottom: 1px solid var(--border-color);
  background-color: var(--bg-color-secondary);
}

.settings-header h2 {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
}

.settings-body {
  flex: 1;
  display: flex;
  overflow: hidden;
}

.settings-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  background-color: var(--bg-color);
  overflow: hidden;
}

.category-header {
  padding: 16px 24px 0;
}

.category-header h3 {
  margin: 0;
  font-size: 20px;
  font-weight: 600;
}

.category-scroll-area {
  flex: 1;
  overflow-y: auto;
  padding: 16px 24px 24px;
}

.loading-state,
.error-state,
.empty-hint,
.static-panel {
  padding: 32px;
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 16px;
}

.static-row {
  display: flex;
  align-items: center;
  gap: 12px;
}
</style>

