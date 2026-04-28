<template>
  <WindowFrame>
    <SearchBar v-if="searchStore.keepSearchBar" />

    <ResultList
      v-if="searchStore.sessionMode === 'search' && !searchStore.isIdle"
      :results="searchStore.results"
      :selected-index="searchStore.selectedIndex"
      @select="searchStore.selectedIndex = $event"
      @confirm="searchStore.doConfirm($event)"
    />

    <EmptyState v-else-if="searchStore.isIdle && searchStore.sessionMode !== 'plugin'" />

    <PluginPanelHost
      v-else-if="searchStore.sessionMode === 'plugin' && searchStore.panelType"
      :panel-type="searchStore.panelType"
      :panel-data="searchStore.panelData"
    />

    <Footer
      v-if="!searchStore.isIdle && searchStore.keepSearchBar"
      :result-count="searchStore.results.length"
    />
  </WindowFrame>
</template>

<script setup lang="ts">
import { onMounted } from 'vue'
import WindowFrame from '../components/layout/WindowFrame.vue'
import SearchBar from '../components/search/SearchBar.vue'
import ResultList from '../components/results/ResultList.vue'
import EmptyState from '../components/panel/EmptyState.vue'
import PluginPanelHost from '../components/panel/PluginPanelHost.vue'
import Footer from '../components/layout/Footer.vue'
import { useSearchStore } from '../stores/search-store'
import { useKeyboard } from '../composables/useKeyboard'
import { useWindowResize } from '../composables/useWindowResize'

const searchStore = useSearchStore()

useKeyboard()
useWindowResize()

onMounted(() => {
  searchStore.fetchCandidatesCount()
})
</script>

<style scoped>
/* SearchView is composed entirely of child components */
</style>
