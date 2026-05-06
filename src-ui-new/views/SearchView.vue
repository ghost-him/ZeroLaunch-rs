<template>
  <WindowFrame>
    <SearchBar v-if="searchStore.keepSearchBar" />

    <LoadingIndicator v-if="searchStore.isSearching" />

    <ResultList
      v-if="!searchStore.isSearching && searchStore.sessionMode === 'search' && !searchStore.isIdle"
      :results="searchStore.results"
      :selected-index="searchStore.selectedIndex"
      :action-index="searchStore.selectedActionIndex"
      @select="searchStore.selectedIndex = $event"
      @confirm="(idx: number, actionIdx?: number) => searchStore.doConfirm(idx, actionIdx !== undefined ? searchStore.results[idx]?.actions[actionIdx]?.id : undefined)"
      @context-action="(idx: number, actionId: string) => searchStore.doConfirm(idx, actionId)"
    />

    <EmptyState v-else-if="!searchStore.isSearching && searchStore.isIdle && searchStore.sessionMode !== 'plugin'" />

    <PluginPanelHost
      v-else-if="!searchStore.isSearching && searchStore.sessionMode === 'plugin' && searchStore.panelType"
      :panel-type="searchStore.panelType"
      :panel-data="searchStore.panelData"
    />

    <Footer
      v-if="!searchStore.isIdle && searchStore.keepSearchBar"
      :result-count="searchStore.results.length"
      :session-mode="searchStore.sessionMode"
      :panel-type="searchStore.panelType"
    />
  </WindowFrame>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue'
import WindowFrame from '../components/layout/WindowFrame.vue'
import SearchBar from '../components/search/SearchBar.vue'
import ResultList from '../components/results/ResultList.vue'
import EmptyState from '../components/panel/EmptyState.vue'
import PluginPanelHost from '../components/panel/PluginPanelHost.vue'
import Footer from '../components/layout/Footer.vue'
import LoadingIndicator from '../components/common/LoadingIndicator.vue'
import { useSearchStore } from '../stores/search-store'
import { useKeyboard } from '../composables/useKeyboard'
import { useWindowResize } from '../composables/useWindowResize'
import { useSearch } from '../composables/useSearch'
import { onConfigChanged, onInstallationEvent } from '../bridge/events'

const searchStore = useSearchStore()
const { cleanup } = useSearch()

useKeyboard()
useWindowResize()

let unlistenConfig: (() => void) | null = null
let unlistenInstall: (() => void) | null = null

onMounted(async () => {
  searchStore.fetchCandidatesCount()

  unlistenConfig = await onConfigChanged((payload) => {
    // DataSource/KeywordOptimizer 变更时刷新候选项
    if (payload.componentType === 'DataSource' || payload.componentType === 'KeywordOptimizer') {
      searchStore.refreshCandidates()
    }
  })

  unlistenInstall = await onInstallationEvent(() => {
    searchStore.refreshCandidates()
  })
})

onUnmounted(() => {
  cleanup()
  unlistenConfig?.()
  unlistenInstall?.()
})
</script>

<style scoped>
/* SearchView is composed entirely of child components */
</style>
