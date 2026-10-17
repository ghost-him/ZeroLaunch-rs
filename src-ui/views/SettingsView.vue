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
        <DebugView v-else-if="selectedId === 'category_debug'" />

        <!-- Unified plugin management (builtin + third-party) -->
        <PluginsManagement v-else-if="selectedCategory?.type === 'plugins-page'" />

        <!-- Plugin Market (GitHub topic discovery) -->
        <PluginMarket v-else-if="selectedId === 'category_plugin_market'" />

        <!-- About -->
        <AboutView v-else-if="selectedId === 'category_about'" :version="version" />

        <!-- AI 助手（独立问答页） -->
        <AssistantView
          v-else-if="selectedId === 'category_assistant'"
          :navigation="assistantNavigation"
        />

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
              :group-tabs="selectedCategory.groupTabs"
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
import { ref, computed, nextTick, onMounted, provide } from 'vue'
import { NButton, NSpin, NText, useNotification } from 'naive-ui'
import SettingsSidebar from '../components/settings/SettingsSidebar.vue'
import CategoryViewList from '../components/settings/CategoryViewList.vue'
import CategoryViewPipeline from '../components/settings/CategoryViewPipeline.vue'
import CategoryViewTabs from '../components/settings/CategoryViewTabs.vue'
import DebugView from './DebugView.vue'
import AboutView from './AboutView.vue'
import AssistantView from './settings/AssistantView.vue'
import PluginsManagement from './settings/PluginsManagement.vue'
import PluginMarket from './settings/PluginMarket.vue'
import { useConfigStore } from '../stores/config-store'
import { buildSidebarItems } from '../utils/settingsSidebar'
import { useI18n } from 'vue-i18n'
import { registerErrorHandler } from '../bridge/commands'
import { configGetVersion } from '../bridge/commands'
import type { BridgeError } from '../bridge/commands'
import { formatErrorMessage } from '../utils/errors'
import { FIELD_REVEAL_KEY, type FieldRevealTarget } from '../utils/fieldReveal'
import { resolveText } from '../i18n'
import type { ComponentInfo, HelpNavigation } from '../bridge/contract'

const configStore = useConfigStore()
const notification = useNotification()
const { t } = useI18n()

const loading = ref(true)
const loadErr = ref<string | null>(null)
const selectedId = ref<string | null>('category_core')
const version = ref('')

function getComponentsList(): ComponentInfo[] {
  return Object.values(configStore.components)
}

const isDebugMode = computed(() => {
  const settings = configStore.settings['general-config'] as Record<string, unknown> | undefined
  return (settings?.is_debug_mode as boolean) ?? false
})

const sidebarItems = computed(() => buildSidebarItems(getComponentsList(), isDebugMode.value, t))

/** 助手用的「设置页路径」：分类 → 组件，供模型给出逐步导航（分类由本页拥有，故在此拼装）。 */
const assistantNavigation = computed<HelpNavigation[]>(() => {
  const root = t('settings.title')
  const entries: HelpNavigation[] = []
  const covered = new Set<string>()
  for (const category of sidebarItems.value) {
    for (const component of category.components ?? []) {
      covered.add(component.componentId)
      entries.push({
        componentId: component.componentId,
        path: `${root} › ${category.label} › ${resolveText(component.componentName)}`,
      })
    }
  }
  // 第三方插件组件不在分类里（配置入口在插件管理页），统一标到「插件」
  for (const component of getComponentsList()) {
    if (covered.has(component.componentId)) continue
    entries.push({
      componentId: component.componentId,
      path: `${root} › ${t('settings.sidebar.plugins')} › ${resolveText(component.componentName)}`,
    })
  }
  return entries
})

/** 待揭示的设置位置：由设置助手跳转触发，分类视图与表单消费。 */
const revealTarget = ref<FieldRevealTarget | null>(null)

/**
 * 请求揭示某个设置位置：先切到该组件所在的分类，再由分类视图与表单滚动到字段。
 * 参数：componentId - 组件 id；fieldKey - 字段 key（空串表示只定位到组件）。
 * 返回是否找到了可显示该组件的分类；定位是一次性的，nextTick（本轮 patch 完成后）
 * 即清空 target，避免下次手动进入该分类时重复切 tab/滚动。
 */
function requestReveal(componentId: string, fieldKey: string): boolean {
  const category = sidebarItems.value.find((item) =>
    item.components?.some((component) => component.componentId === componentId),
  )
  if (category) {
    selectedId.value = category.key
  }
  revealTarget.value = { componentId, fieldKey }
  void nextTick(() => {
    revealTarget.value = null
  })
  return category !== undefined
}

provide(FIELD_REVEAL_KEY, { target: revealTarget, request: requestReveal })

const selectedCategory = computed(() => {
  if (!selectedId.value) return null
  return sidebarItems.value.find((item) => item.key === selectedId.value) ?? null
})

async function init() {
  loading.value = true
  loadErr.value = null
  try {
    const [ loadedVersion ] = await Promise.all([
      configGetVersion(),
      configStore.loadAllComponents(),
      configStore.getSettings('general-config'),
    ])
    version.value = loadedVersion
  } catch (e) {
    loadErr.value = formatErrorMessage(e)
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
  min-width: 0;
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
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow-y: auto;
  padding: 16px 24px 0;
}

.loading-state,
.error-state,
.empty-hint {
  padding: 32px;
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 16px;
}
</style>

