<template>
  <div class="search-bar-wrapper" @contextmenu.prevent="onContextMenu">
    <!-- 行内参数模式：显示触发词前缀 + 参数输入 -->
    <div v-if="searchStore.inlineParamState" class="inline-param-bar">
      <span class="trigger-prefix">{{ searchStore.inlineParamState.triggerKeyword }}</span>
      <input
        ref="paramInputRef"
        v-model="searchStore.inlineParamState.paramInput"
        class="param-input"
        :placeholder="'输入 ' + searchStore.inlineParamState.userArgCount + ' 个参数（空格分隔，\\ 转义空格）'"
        autofocus
      />
    </div>

    <!-- 正常搜索模式 -->
    <n-input
      v-else
      ref="inputRef"
      v-model:value="searchStore.query"
      type="text"
      :placeholder="themeStore.searchBarPlaceholder"
      :autofocus="true"
      size="large"
      @update:value="onInput"
    />
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
const paramInputRef = ref<HTMLInputElement | null>(null)

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
  if (paramInputRef.value) {
    paramInputRef.value.focus()
  } else {
    inputRef.value?.focus()
  }
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
  padding: 0 24px;
  display: flex;
  align-items: center;
  position: relative;
  z-index: 10;
  /* Soft hierarchical shadow indicating separation without hard lines */
  box-shadow: var(--shadow-header);
}

/* 行内参数模式 */
.inline-param-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
}

.trigger-prefix {
  font-size: var(--font-size-xl);
  font-weight: 600;
  color: var(--accent-color);
  white-space: nowrap;
  padding: 2px 8px;
  background: var(--accent-bg-subtle);
  border-radius: 4px;
}

.param-input {
  flex: 1;
  height: 100%;
  border: none;
  background: transparent;
  color: var(--text-primary);
  font-size: var(--font-size-xl);
  font-family: var(--search-bar-font-family);
  outline: none;
  caret-color: var(--accent-color);
}

.param-input::placeholder {
  color: var(--text-secondary);
  font-size: var(--font-size-md);
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
  --n-height: 100% !important;
  --n-caret-color: var(--accent-color) !important;
  --n-padding-left: 0 !important; /* Flush with wrapper padding */
  --n-font-family: var(--search-bar-font-family) !important;
}
.search-bar-wrapper :deep(.n-input .n-input-wrapper) {
  padding-top: 10px;
  padding-bottom: 10px;
}
</style>
