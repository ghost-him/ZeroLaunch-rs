<template>
  <div
    class="everything-panel"
    :style="{
      height: uiConfig.result_item_height * appConfig.search_result_count + 'px',
      maxHeight: uiConfig.result_item_height * appConfig.search_result_count + 'px',
    }"
  >
    <div
      v-if="results.length === 0 && !isSearching"
      class="no-results"
      :style="{
        color: uiConfig.item_font_color,
        fontFamily: uiConfig.result_item_font_family,
        fontSize: Math.round(uiConfig.result_item_height * 0.4) + 'px'
      }"
    >
      {{ t('app.no_results') }}
    </div>
    <div
      v-else
      ref="resultsListRef"
      class="results-list"
    >
      <div
        v-for="(item, index) in results"
        :key="item[0]"
        class="result-item"
        :class="{ 'selected': selectedIndex === index }"
        :style="{
          '--hover-color': hoverColor,
          '--selected-color': uiConfig.selected_item_color,
          height: uiConfig.result_item_height + 'px',
        }"
        @click="handleItemClick(index)"
      >
        <div
          class="icon"
          :style="{
            width: Math.round(uiConfig.result_item_height * layoutConstants.iconSizeRatio) + 'px',
            height: Math.round(uiConfig.result_item_height * layoutConstants.iconSizeRatio) + 'px',
            marginLeft: Math.round(uiConfig.result_item_height * layoutConstants.iconMarginRatio) + 'px',
            marginRight: Math.round(uiConfig.result_item_height * layoutConstants.iconMarginRatio) + 'px',
          }"
        >
          <img
            src="/tauri.svg"
            class="custom-image"
            alt="icon"
          >
        </div>
        <div class="item-info">
          <div
            class="item-name"
            :style="{
              fontSize: Math.round(uiConfig.result_item_height * uiConfig.item_font_size * layoutConstants.fontSizeRatio) + 'px',
              fontFamily: uiConfig.result_item_font_family,
              color: uiConfig.item_font_color
            }"
          >
            {{ item[1] }}
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'
import type { UIConfig, AppConfig } from '../../api/remote_config_types'

const props = defineProps<{
    searchText: string;
    uiConfig: UIConfig;
    appConfig: AppConfig;
    hoverColor: string;
}>()

const { t } = useI18n()

const layoutConstants = {
    iconSizeRatio: 0.6,
    iconMarginRatio: 0.2,
    fontSizeRatio: 0.01,
}

const results = ref<Array<[number, string]>>([])
const selectedIndex = ref(0)
const isSearching = ref(false)
const pendingSearchText = ref<string | null>(null)
const resultsListRef = ref<HTMLElement | null>(null)

const performSearch = async (text: string) => {
    if (isSearching.value) {
        pendingSearchText.value = text
        return
    }

    isSearching.value = true
    try {
        const searchResults: Array<[number, string]> = await invoke('handle_everything_search', { searchText: text })
        results.value = searchResults
        selectedIndex.value = 0
        if (resultsListRef.value) {
            resultsListRef.value.scrollTop = 0
        }
    } catch (error) {
        console.error('Everything search failed:', error)
    } finally {
        isSearching.value = false
        if (pendingSearchText.value !== null) {
            const nextText = pendingSearchText.value
            pendingSearchText.value = null
            if (nextText !== text) {
                performSearch(nextText)
            }
        }
    }
}

watch(() => props.searchText, (newText) => {
    performSearch(newText)
})

const handleItemClick = (index: number) => {
    selectedIndex.value = index
    launchItem(index)
}

const launchItem = async (index: number) => {
    const item = results.value[index]
    if (item) {
        try {
            await invoke('launch_everything_item', { path: item[1] })
        } catch (error) {
            console.error('Failed to launch everything item:', error)
        }
    }
}

const moveSelection = (direction: number) => {
    const newIndex = selectedIndex.value + direction
    if (newIndex >= 0 && newIndex < results.value.length) {
        selectedIndex.value = newIndex
        scrollToSelected()
    }
}

const scrollToSelected = () => {
    if (!resultsListRef.value) return
    const selectedEl = resultsListRef.value.children[selectedIndex.value] as HTMLElement
    if (selectedEl) {
        selectedEl.scrollIntoView({ block: 'nearest' })
    }
}

defineExpose({
    moveSelection,
    launchSelected: () => launchItem(selectedIndex.value),
    resultsCount: () => results.value.length,
})

onMounted(() => {
    if (props.searchText) {
        performSearch(props.searchText)
    }
})
</script>

<style scoped>
.everything-panel {
    overflow-y: auto;
    scrollbar-width: thin;
    scrollbar-color: rgba(0, 0, 0, 0.2) transparent;
}

.everything-panel::-webkit-scrollbar {
    width: 6px;
}

.everything-panel::-webkit-scrollbar-track {
    background: transparent;
}

.everything-panel::-webkit-scrollbar-thumb {
    background: rgba(0, 0, 0, 0.2);
    border-radius: 3px;
}

.everything-panel::-webkit-scrollbar-thumb:hover {
    background: rgba(0, 0, 0, 0.4);
}

.no-results {
    display: flex;
    justify-content: center;
    align-items: center;
    height: 100%;
    opacity: 0.6;
}

.result-item {
    display: flex;
    align-items: center;
    cursor: pointer;
    transition: background-color 0.2s;
}

.result-item:hover {
    background-color: var(--hover-color);
}

.result-item.selected {
    background-color: var(--selected-color);
}

.icon {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
}

.custom-image {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    border-radius: 6px;
}

.item-info {
    display: flex;
    align-items: center;
    min-width: 0;
    overflow: hidden;
    height: 100%;
    flex: 1;
}

.item-name {
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    width: 100%;
}
</style>
