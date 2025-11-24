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
      {{ t('everything.no_results') }}
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
            :src="iconMap.get(item[1]) || '/tauri.svg'"
            class="custom-image"
            alt="icon"
          >
        </div>
        <div class="item-content">
          <div
            class="file-name"
            :style="{
              fontSize: Math.round(uiConfig.result_item_height * uiConfig.item_font_size * layoutConstants.fontSizeRatio) + 'px',
              fontFamily: uiConfig.result_item_font_family,
              color: uiConfig.item_font_color,
              fontWeight: '600',
            }"
          >
            {{ getFileName(item[1]) }}
          </div>
          <div
            class="file-path"
            :style="{
              fontSize: Math.round(uiConfig.result_item_height * uiConfig.item_font_size * layoutConstants.fontSizeRatio * 0.65) + 'px',
              fontFamily: uiConfig.result_item_font_family,
              color: getPathColor(),
            }"
          >
            {{ getDirectoryPath(item[1]) }}
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from 'vue-i18n'
import { getColorWithReducedOpacity } from '../../utils/color'
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
const iconMap = ref<Map<string, string>>(new Map())

// 从完整路径中提取文件名（包含扩展名）
const getFileName = (fullPath: string): string => {
    const parts = fullPath.split('\\')
    return parts[parts.length - 1] || fullPath
}

// 从完整路径中提取目录路径
const getDirectoryPath = (fullPath: string): string => {
    const parts = fullPath.split('\\')
    if (parts.length <= 1) return fullPath
    return parts.slice(0, -1).join('\\')
}

// 计算路径颜色（使用统一的 color.ts 函数）
const getPathColor = (): string => {
    return getColorWithReducedOpacity(props.uiConfig.item_font_color, 0.6)
}

const loadIcons = async () => {
    // 创建新的 Map
    const newIconMap = new Map<string, string>()
    
    // 并行加载所有图标
    const promises = results.value.map(async (item) => {
        const path = item[1]
        try {
            const iconData = await invoke<number[]>('get_everything_icon', { path })
            if (iconData && iconData.length > 0) {
                const blob = new Blob([new Uint8Array(iconData)], { type: 'image/png' })
                const url = URL.createObjectURL(blob)
                newIconMap.set(path, url)
            }
        } catch (e) {
            // console.error('Failed to load icon for', path, e)
        }
    })
    
    await Promise.all(promises)
    
    // 释放旧的 URL
    iconMap.value.forEach(url => URL.revokeObjectURL(url))
    iconMap.value = newIconMap
}

onUnmounted(() => {
    iconMap.value.forEach(url => URL.revokeObjectURL(url))
})

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
        // 加载图标
        loadIcons()
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
    flex-shrink: 0;
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

.item-content {
    display: flex;
    flex-direction: column;
    justify-content: center;
    min-width: 0;
    overflow: hidden;
    flex: 1;
    padding-right: 12px;
    height: 100%;
}

.file-name {
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    width: 100%;
    line-height: 1.2;
}

.file-path {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    width: 100%;
    opacity: 0.8;
    line-height: 1.2;
}
</style>
