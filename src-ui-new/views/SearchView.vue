<template>
  <WindowFrame>
    <SearchBar v-if="searchStore.keepSearchBar" ref="searchBarRef" @contextmenu="onShowCtxMenu" />

    <ResultList
      v-if="searchStore.sessionMode === 'search' && !searchStore.isIdle"
      :results="searchStore.results"
      :selected-index="searchStore.selectedIndex"
      @select="searchStore.selectedIndex = $event"
      @confirm="(idx: number) => searchStore.doConfirm(idx)"
      @context-action="(idx: number, actionId: string) => searchStore.doConfirm(idx, actionId)"
      @contextmenu="onShowCtxMenu"
    />

    <PluginPanelHost
      v-else-if="searchStore.sessionMode === 'plugin' && searchStore.panelType"
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
import { ref, nextTick } from 'vue'
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
import { useKeyboard } from '../composables/useKeyboard'
import { useWindowResize } from '../composables/useWindowResize'
import { useSearch } from '../composables/useSearch'
import { onConfigChanged, onInstallationEvent } from '../bridge/events'
import { registerErrorHandler } from '../bridge/commands'
import type { BridgeError } from '../bridge/commands'

const searchStore = useSearchStore()
useSearch()
const notification = useNotification()

useKeyboard()
const { resizeWindow } = useWindowResize()

// ---- 集中管理的右键菜单状态 ----
const ctxVisible = ref(false)
const ctxX = ref(0)
const ctxY = ref(0)
const ctxItems = ref<CtxItem[]>([])

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

  unlistenConfig = await onConfigChanged((payload) => {
    // DataSource/KeywordOptimizer 变更时刷新候选项
    if (payload.componentType === 'DataSource' || payload.componentType === 'KeywordOptimizer') {
      searchStore.refreshCandidates()
    }
    // 外观配置变更时重新计算窗口尺寸（宽度/高度/字体可能变化）
    if (payload.componentId === 'appearance') {
      nextTick(() => resizeWindow())
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
