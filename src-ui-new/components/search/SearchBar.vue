<template>
  <div class="search-bar-wrapper" @contextmenu.prevent="onContextMenu">
    <n-input
      ref="inputRef"
      v-model:value="searchStore.query"
      type="text"
      :placeholder="themeStore.searchBarPlaceholder"
      :autofocus="true"
      size="large"
      @update:value="onInput"
    >
      <!-- 无边框无图标的纯净设计 -->
    </n-input>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, nextTick } from 'vue'
import { NInput } from 'naive-ui'
import { useSearch } from '../../composables/useSearch'
import { useSettings } from '../../composables/useSettings'
import { useThemeStore } from '../../stores/theme-store'
import { useSearchStore } from '../../stores/search-store'
import type { CtxItem } from '../layout/ContextMenu.vue'

const { handleInput } = useSearch()
const { openSettings } = useSettings()
const themeStore = useThemeStore()
const searchStore = useSearchStore()

const inputRef = ref<InstanceType<typeof NInput> | null>(null)

function onInput(value: string) {
  handleInput(value)
}

// ---- 右键菜单（事件委托给 SearchView） ----
const emit = defineEmits<{
  (e: 'contextmenu', x: number, y: number, items: CtxItem[]): void
}>()

function onContextMenu(e: MouseEvent) {
  const items: CtxItem[] = [
    {
      key: 'open-settings',
      label: '打开设置',
      action: () => openSettings(),
    },
  ]
  emit('contextmenu', e.clientX, e.clientY, items)
}

// 暴露 focusInput 方法供外部恢复焦点
function focusInput() {
  inputRef.value?.focus()
}

onMounted(async () => {
  await nextTick()
  inputRef.value?.focus()
})

defineExpose({ focusInput })
</script>

<style scoped>
.search-bar-wrapper {
  height: var(--search-bar-height);
  flex-shrink: 0;
  padding: 16px 24px; /* Larger horizontal padding */
  display: flex;
  align-items: center;
  position: relative;
  z-index: 10;
  /* Soft hierarchical shadow indicating separation without hard lines */
  box-shadow: var(--shadow-header);
}

.search-bar-wrapper :deep(.n-input) {
  --n-border: transparent !important;
  --n-border-hover: transparent !important;
  --n-border-focus: transparent !important;
  --n-box-shadow-focus: transparent !important;
  --n-color: transparent !important;
  --n-color-focus: transparent !important;
  --n-text-color: var(--text-primary);
  --n-placeholder-color: var(--text-secondary);
  --n-font-size: var(--font-size-xl) !important;
  --n-height: 40px !important;
  --n-caret-color: var(--accent-color) !important;
  --n-padding-left: 0 !important; /* Flush with wrapper padding */
  --n-font-family: var(--search-bar-font-family) !important;
}
</style>
