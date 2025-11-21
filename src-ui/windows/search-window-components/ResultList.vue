<template>
  <div class="results-list" :class="{ 'scroll-mode': isScrollMode }" ref="resultsListRef" :style="listStyle">
    <div v-for="(item, index) in menuItems" :key="index" class="result-item"
      @click="(event) => handleItemClick(index, event.ctrlKey)" :class="{ 'selected': selectedIndex === index }"
      @contextmenu.prevent="(event) => handleContextMenu(index, event)" :style="{
        '--hover-color': hoverColor,
        '--selected-color': uiConfig.selected_item_color,
        height: uiConfig.result_item_height + 'px',
      }">
      <div class="icon" :style="{
        width: Math.round(uiConfig.result_item_height * 0.6) + 'px',
        height: Math.round(uiConfig.result_item_height * 0.6) + 'px',
        marginLeft: Math.round(uiConfig.result_item_height * 0.2) + 'px',
        marginRight: Math.round(uiConfig.result_item_height * 0.2) + 'px',
      }">
        <img :src="menuIcons[index]" class="custom-image" alt="icon">
      </div>
      <div class="item-info">
        <div class="item-name" v-html="item" :style="{
          fontSize: Math.round(uiConfig.result_item_height * uiConfig.item_font_size / 100) + 'px',
          fontFamily: uiConfig.result_item_font_family,
          color: uiConfig.item_font_color
        }"></div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue';
import type { UIConfig, AppConfig } from '../../api/remote_config_types';

const props = defineProps<{
  menuItems: string[];
  menuIcons: string[];
  selectedIndex: number;
  uiConfig: UIConfig;
  appConfig: AppConfig;
  hoverColor: string;
  isScrollMode: boolean;
}>();

const emit = defineEmits<{
  (e: 'item-click', index: number, ctrlKey: boolean): void;
  (e: 'item-contextmenu', index: number, event: MouseEvent): void;
}>();

const resultsListRef = ref<HTMLElement | null>(null);

const scrollModeMaxHeight = computed(() => {
  return `${props.appConfig.scroll_threshold * props.uiConfig.result_item_height}px`;
});

const listStyle = computed(() => {
    if (props.isScrollMode) {
        return {
            maxHeight: `${props.appConfig.scroll_threshold * props.uiConfig.result_item_height}px`,
            overflowY: 'auto' as const
        }
    }
    return {};
});

const handleItemClick = (index: number, ctrlKey: boolean) => {
  emit('item-click', index, ctrlKey);
};

const handleContextMenu = (index: number, event: MouseEvent) => {
  emit('item-contextmenu', index, event);
};

defineExpose({
  resultsListRef
});
</script>

<style scoped>
.results-list {
  overflow-y: auto;
  min-height: 0;
  scrollbar-width: thin;
  scrollbar-color: rgba(0, 0, 0, 0.2) transparent;
}

.results-list.scroll-mode {
  max-height: v-bind(scrollModeMaxHeight);
  overflow-y: auto;
}

.results-list::-webkit-scrollbar {
  width: 6px;
}

.results-list::-webkit-scrollbar-track {
  background: transparent;
}

.results-list::-webkit-scrollbar-thumb {
  background: rgba(0, 0, 0, 0.2);
  border-radius: 3px;
}

.results-list::-webkit-scrollbar-thumb:hover {
  background: rgba(0, 0, 0, 0.4);
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
  image-rendering: -webkit-optimize-contrast;
  transform: translateZ(0);
}

.item-info {
  display: flex;
  align-items: center;
  min-width: 0;
  overflow: hidden;
  height: 100%;
}

.item-name {
  font-weight: 500;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  width: 100%;
  line-height: normal;
}

:deep(mark) {
  background-color: transparent;
  color: inherit;
  font-weight: 700;
  padding: 0;
}
</style>
