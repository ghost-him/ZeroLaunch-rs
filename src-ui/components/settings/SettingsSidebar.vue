<template>
  <div class="settings-sidebar">
    <SettingsSearch
      ref="searchRef"
      :sidebar-items="sidebarItems"
      :navigation="navigation"
      @navigate="onNavigate"
      @ask-ai="onAskAi"
    />

    <div class="sidebar-menu">
      <n-menu
        :options="menuOptions"
        :value="selectedId"
        @update:value="onSelect"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { NMenu } from 'naive-ui'
import type { MenuOption } from 'naive-ui'
import type { HelpNavigation } from '../../bridge/contract'
import type { SidebarCategory } from '../../utils/settingsSidebar'
import { renderSidebarIcon } from './sidebarIcons'
import SettingsSearch from './SettingsSearch.vue'

/** 搜索组件暴露的关闭入口（点击菜单时收起浮层）。 */
interface SettingsSearchExpose {
  closePanel: () => void
}

const props = defineProps<{
  sidebarItems: SidebarCategory[]
  selectedId: string | null
  /** 组件 id → 完整设置页路径（与问 AI 导航共用同一份投影）。 */
  navigation: HelpNavigation[]
}>()

const emit = defineEmits<{
  (e: 'select', key: string): void
  (e: 'navigate', componentId: string, fieldKey: string): void
  (e: 'ask-ai', query: string): void
}>()

const searchRef = ref<SettingsSearchExpose | null>(null)

const menuOptions = computed<MenuOption[]>(() => {
  return props.sidebarItems.map(item => ({
    label: item.label,
    key: item.key,
    icon: renderSidebarIcon(item.icon)
  }))
})

/** 转发搜索命中：分类切换与字段定位由设置页编排。 */
function onNavigate(componentId: string, fieldKey: string): void {
  emit('navigate', componentId, fieldKey)
}

/** 转发问 AI 请求：问题扩写与页面切换由设置页编排。 */
function onAskAi(query: string): void {
  emit('ask-ai', query)
}

function onSelect(key: string): void {
  searchRef.value?.closePanel()
  emit('select', key)
}
</script>

<style scoped>
.settings-sidebar {
  width: 200px;
  flex-shrink: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  border-right: 1px solid var(--border-color);
}

.sidebar-menu {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 4px 0 8px;
}
</style>
