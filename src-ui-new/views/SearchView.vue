<template>
  <WindowFrame>
    <!-- 全页面插件模式：隐藏所有默认 UI -->
    <template v-if="uiMode === 'full_page_plugin'">
      <!-- 全页面插件内容由插件自定义面板渲染 -->
      <PluginPanelHost
        v-if="searchStore.panelType"
        :panel-type="searchStore.panelType"
        :panel-data="searchStore.panelData"
      />
    </template>

    <!-- 其他模式：显示搜索栏 + 结果 -->
    <template v-else>
      <SearchBar v-if="searchStore.keepSearchBar" ref="searchBarRef" @contextmenu="onShowCtxMenu" />

      <!-- 参数面板模式 -->
      <ParamPanel v-if="uiMode === 'param_panel'" />

      <!-- 搜索结果列表（搜索模式或行内参数模式） -->
      <ResultList
        v-if="(uiMode === 'search' || uiMode === 'inline_param') && !searchStore.isIdle"
        :results="searchStore.results"
        :selected-index="searchStore.selectedIndex"
        @select="searchStore.selectedIndex = $event"
        @confirm="(idx: number) => searchStore.doConfirm(idx)"
        @context-action="(idx: number, actionId: string) => searchStore.doConfirm(idx, actionId)"
        @contextmenu="onShowCtxMenu"
      />

      <!-- 行内插件模式 -->
      <PluginPanelHost
        v-if="uiMode === 'inline_plugin' && searchStore.panelType"
        :panel-type="searchStore.panelType"
        :panel-data="searchStore.panelData"
      />

      <Footer
        v-if="!searchStore.isIdle && searchStore.keepSearchBar"
        :result-count="searchStore.results.length"
        :session-mode="searchStore.sessionMode"
        :panel-type="searchStore.panelType"
        :actions="searchStore.selectedItem?.actions ?? []"
        :selected-action-index="searchStore.selectedActionIndex"
        @action-execute="(actionId: string) => searchStore.doConfirm(undefined, actionId)"
      />
    </template>
  </WindowFrame>

  <!-- 集中管理的唯一 ContextMenu 实例 -->
  <ContextMenu
    :visible="ctxVisible"
    :x="ctxX"
    :y="ctxY"
    :items="ctxItems"
    @close="onCtxClose"
  />
</template>

<script setup lang="ts">
import { provide, ref, nextTick } from 'vue'
import { onMounted, onUnmounted } from 'vue'
import { useNotification } from 'naive-ui'
import WindowFrame from '../components/layout/WindowFrame.vue'
import SearchBar from '../components/search/SearchBar.vue'
import ResultList from '../components/results/ResultList.vue'
import PluginPanelHost from '../components/panel/PluginPanelHost.vue'
import Footer from '../components/layout/Footer.vue'
import ContextMenu from '../components/layout/ContextMenu.vue'
import type { CtxItem } from '../components/layout/ContextMenu.vue'

import { useSearchStore } from '../stores/search-store'
import { useKeyboardRouter } from '../composables/useKeyboardRouter'
import { useWindowResize } from '../composables/useWindowResize'
import { useSearch } from '../composables/useSearch'
import ParamPanel from '../components/search/ParamPanel.vue'
import { onConfigChanged, onInstallationEvent } from '../bridge/events'
import { registerErrorHandler, configGetSettings } from '../bridge/commands'
import type { BridgeError } from '../bridge/commands'

const searchStore = useSearchStore()
useSearch()
const notification = useNotification()

const { uiMode } = useKeyboardRouter()
const { resizeWindow } = useWindowResize()

// ---- 集中管理的右键菜单状态 ----
const ctxVisible = ref(false)
const ctxX = ref(0)
const ctxY = ref(0)
const ctxItems = ref<CtxItem[]>([])

const isDragEnabled = ref(false)
provide('isDragEnabled', isDragEnabled)

async function loadDragSetting() {
  try {
    const settings = (await configGetSettings('window-behavior')) as Record<string, unknown> | null
    isDragEnabled.value = (settings?.is_enable_drag_window as boolean) ?? false
    console.log('[SearchView] Drag setting loaded:', isDragEnabled.value)
  } catch (e) {
    console.warn('[SearchView] Failed to load drag setting:', e)
  }
}

const searchBarRef = ref<InstanceType<typeof SearchBar> | null>(null)

function onShowCtxMenu(x: number, y: number, items: CtxItem[]) {
  ctxX.value = x
  ctxY.value = y
  ctxItems.value = items
  ctxVisible.value = true
}

function onCtxClose() {
  ctxVisible.value = false
  // 关闭菜单后恢复搜索栏焦点
  nextTick(() => {
    searchBarRef.value?.focusInput()
  })
}

let unlistenConfig: (() => void) | null = null
let unlistenInstall: (() => void) | null = null

onMounted(async () => {
  // 注册全局错误处理器（必须在 n-notification-provider 后代中调用）
  registerErrorHandler((error: BridgeError) => {
    notification.error({
      title: error.code,
      content: error.message,
      duration: 5000,
    })
  })

  searchStore.fetchCandidatesCount()
  await loadDragSetting()

  unlistenConfig = await onConfigChanged((payload) => {
    // DataSource/KeywordOptimizer 变更时刷新候选项
    if (payload.componentType === 'DataSource' || payload.componentType === 'KeywordOptimizer') {
      searchStore.refreshCandidates()
    }
    // 外观配置变更时重新计算窗口尺寸（宽度/高度/字体可能变化）
    if (payload.componentId === 'appearance') {
      nextTick(() => resizeWindow())
    }
    // 窗口行为变更时重新加载拖动设置
    if (payload.componentId === 'window-behavior') {
      loadDragSetting()
    }
  })

  unlistenInstall = await onInstallationEvent(() => {
    searchStore.refreshCandidates()
  })
})

onUnmounted(() => {
  unlistenConfig?.()
  unlistenInstall?.()
})
</script>

<style scoped>
/* SearchView is composed entirely of child components */
</style>
