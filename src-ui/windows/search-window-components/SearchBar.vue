<template>
  <div
    class="search-input"
    :style="{ background: props.uiConfig.search_bar_background_color, height: props.uiConfig.search_bar_height + 'px' }"
    @contextmenu.prevent="onContextMenu"
    @mousedown="startDrag"
  >
    <span
      class="search-icon"
      :style="{
        marginLeft: Math.round(props.uiConfig.search_bar_height * layoutConstants.iconMarginRatio) + 'px',
        marginRight: Math.round(props.uiConfig.search_bar_height * layoutConstants.iconMarginRatio) + 'px'
      }"
    >
      <svg
        viewBox="0 0 1024 1024"
        :width="Math.round(props.uiConfig.search_bar_height * layoutConstants.iconSizeRatio) + 'px'"
        :height="Math.round(props.uiConfig.search_bar_height * layoutConstants.iconSizeRatio) + 'px'"
      >
        <path
          fill="#999"
          d="M795.904 750.72l124.992 124.928a32 32 0 0 1-45.248 45.248L750.656 795.904a416 416 0 1 1 45.248-45.248zM480 832a352 352 0 1 0 0-704 352 352 0 0 0 0 704z"
        />
      </svg>
    </span>
    <AnimatedInput
      ref="animatedInputRef"
      v-model="modelValue"
      :placeholder="props.appConfig.search_bar_placeholder"
      :font-size="Math.round(props.uiConfig.search_bar_height * props.uiConfig.search_bar_font_size * layoutConstants.fontSizeRatio) + 'px'"
      :color="props.uiConfig.search_bar_font_color"
      :font-family="props.uiConfig.search_bar_font_family"
      :placeholder-color="props.uiConfig.search_bar_placeholder_font_color"
      :dynamic="props.uiConfig.search_bar_animate"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import AnimatedInput from './AnimatedInput.vue'
import type { AppConfig, UIConfig } from '../../api/remote_config_types'
import { getCurrentWindow } from '@tauri-apps/api/window'

const layoutConstants = {
  iconMarginRatio: 0.3,
  iconSizeRatio: 0.4,
  fontSizeRatio: 0.01,
}

const props = defineProps<{
  appConfig: AppConfig;
  uiConfig: UIConfig;
}>()

const modelValue = defineModel<string>({ required: true })
const emit = defineEmits<{
  (e: 'contextmenu', event: MouseEvent): void;
}>()

const animatedInputRef = ref<InstanceType<typeof AnimatedInput> | null>(null)
const realInputRef = computed(() => animatedInputRef.value?.realInputRef)

const onContextMenu = (event: MouseEvent) => {
  emit('contextmenu', event)
}

const startDrag = (e: MouseEvent) => {
  if (!props.appConfig.is_enable_drag_window) return
  if (e.button !== 0) return
  if ((e.target as HTMLElement).tagName.toLowerCase() === 'input') return
  getCurrentWindow().startDragging()
}

const focus = () => {
  animatedInputRef.value?.focus()
}

defineExpose({
  focus,
  realInputRef,
})
</script>

<style scoped>
.search-input {
  box-sizing: border-box;
  display: flex;
  align-items: center;
  border-bottom: 1px solid rgba(0, 0, 0, 0.05);
  flex-shrink: 0;
  flex-grow: 1;
  min-width: 0;
  width: 100%;
}

.search-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}
</style>
 
