<template>
  <div class="search-bar-wrapper" @contextmenu.prevent="onContextMenu">
    <n-input
      ref="inputRef"
      v-model:value="inputValue"
      type="text"
      :placeholder="inputPlaceholder"
      :autofocus="true"
      :disabled="searchStore.paramPanelState !== null"
      size="large"
    >
      <!-- 行内参数模式：触发词作为前缀 chip -->
      <template #prefix v-if="searchStore.inlineParamState">
        <span class="trigger-prefix">{{ searchStore.inlineParamState.triggerKeyword }}</span>
      </template>
    </n-input>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, nextTick } from 'vue'
import { NInput, useNotification } from 'naive-ui'
import { useSearch } from '../../composables/useSearch'
import { useSettings } from '../../composables/useSettings'
import { useThemeStore } from '../../stores/theme-store'
import { useSearchStore } from '../../stores/search-store'
import type { CtxItem } from '../layout/ContextMenu.vue'

const { handleInput } = useSearch()
const { openSettings } = useSettings()
const themeStore = useThemeStore()
const searchStore = useSearchStore()
const notification = useNotification()

const inputRef = ref<InstanceType<typeof NInput> | null>(null)

/// 根据当前模式路由到正确的读写源
const inputValue = computed({
  get: () => {
    if (searchStore.inlineParamState) {
      return searchStore.inlineParamState.paramInput
    }
    return searchStore.query
  },
  set: (val: string) => {
    if (searchStore.inlineParamState) {
      searchStore.inlineParamState.paramInput = val
    } else {
      searchStore.query = val
      handleInput(val)
    }
  },
})

/// 根据当前模式切换 placeholder
const inputPlaceholder = computed(() => {
  if (searchStore.inlineParamState) {
    const n = searchStore.inlineParamState.userArgCount
    return `输入 ${n} 个参数（空格分隔，\\ 转义空格）`
  }
  return themeStore.searchBarPlaceholder
})

// ---- 右键菜单（事件委托给 SearchView） ----
const emit = defineEmits<{
  (e: 'contextmenu', x: number, y: number, items: CtxItem[]): void
}>()

function onContextMenu(e: MouseEvent) {
  const items: CtxItem[] = [
    {
      key: 'refresh-database',
      label: '刷新数据库',
      action: async () => {
        const count = await searchStore.refreshCandidates()
        notification.success({
          title: '数据库已刷新',
          content: `共 ${count} 个候选项`,
          duration: 2000,
        })
      },
    },
    {
      key: 'open-settings',
      label: '打开设置',
      action: () => openSettings(),
    },
  ]
  emit('contextmenu', e.clientX, e.clientY, items)
}

/// 暴露 focusInput 方法供外部恢复焦点
function focusInput() {
  inputRef.value?.focus()
}

// 退出参数面板时恢复搜索栏焦点
watch(
  () => searchStore.paramPanelState,
  (state, prev) => {
    if (prev && !state) {
      nextTick(() => inputRef.value?.focus())
    }
  },
)

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
  box-shadow: var(--shadow-header);
}

.trigger-prefix {
  font-size: var(--font-size-md);
  font-weight: 600;
  color: var(--accent-color);
  white-space: nowrap;
  padding: 2px 8px;
  background: var(--primary-color-alpha);
  border-radius: 4px;
  user-select: none;
}

.search-bar-wrapper :deep(.n-input) {
  --n-border: transparent !important;
  --n-border-hover: transparent !important;
  --n-border-focus: transparent !important;
  --n-box-shadow-focus: transparent !important;
  --n-color: transparent !important;
  --n-color-focus: transparent !important;
  --n-text-color: var(--text-primary);
  --n-text-color-disabled: var(--text-secondary) !important;
  --n-color-disabled: transparent !important;
  --n-border-disabled: transparent !important;
  --n-opacity-disabled: 1 !important;
  --n-placeholder-color: var(--text-secondary);
  --n-font-size: var(--font-size-xl) !important;
  --n-height: 100% !important;
  --n-caret-color: var(--accent-color) !important;
  --n-padding-left: 0 !important;
  --n-font-family: var(--search-bar-font-family) !important;
}
.search-bar-wrapper :deep(.n-input .n-input-wrapper) {
  padding-top: 10px;
  padding-bottom: 10px;
}
</style>
