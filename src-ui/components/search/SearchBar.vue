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
      <!-- trigger 插件面板模式：当前插件图标作为前缀（仅 panel 形态插件展示图标，行内插件不展示） -->
      <template #prefix v-else-if="currentPluginMeta?.mode === 'panel' && currentPluginMeta.icon">
        <img :src="currentPluginMeta.icon" class="plugin-prefix-icon" alt="" />
      </template>
    </n-input>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, nextTick } from 'vue'
import { NInput, useNotification } from 'naive-ui'
import { useI18n } from 'vue-i18n'
import { useSettings } from '../../composables/useSettings'
import { useThemeStore } from '../../stores/theme-store'
import { useSearchStore } from '../../stores/search-store'
import type { CtxItem } from '../layout/ContextMenu.vue'

const { openSettings } = useSettings()
const themeStore = useThemeStore()
const searchStore = useSearchStore()
const notification = useNotification()
const { t } = useI18n()

/// 当前插件面板元数据：插件面板模式下在搜索栏前缀渲染插件图标（仅 panel 形态）。
const currentPluginMeta = computed(() => {
  const id = searchStore.currentPluginId
  if (!id || searchStore.sessionMode !== 'plugin_panel') return null
  return searchStore.pluginMeta[id] ?? null
})

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
      // 输入变化始终触发查询（confirm=false）：
      // - 自动模式（onInput）：后端正常翻译；
      // - 手动模式（onEnter）：后端返回 ready 预览，翻译仅由 Enter 确认查询触发。
      // 查询同时承担路由职责：文本回退到插件触发词之外时，后端回落搜索，前端据此退出面板。
      searchStore.doQuery(val)
    }
  },
})

/// 根据当前模式切换 placeholder
const inputPlaceholder = computed(() => {
  if (searchStore.inlineParamState) {
    const n = searchStore.inlineParamState.userArgCount
    return t('search.paramCountHint', { n })
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
      label: t('search.refreshDatabase'),
      action: async () => {
        const count = await searchStore.refreshCandidates()
        notification.success({
          title: t('search.refreshDone'),
          content: t('search.candidates', { count }),
          duration: 2000,
        })
      },
    },
    {
      key: 'open-settings',
      label: t('search.openSettings'),
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

// 隐藏方式选择面板：打开时让出焦点（键盘由该面板独占，避免输入落进搜索框改写查询），
// 关闭后恢复搜索栏焦点
watch(
  () => searchStore.excludeChooser,
  (state, prev) => {
    if (!prev && state) {
      inputRef.value?.blur()
    } else if (prev && !state) {
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

.plugin-prefix-icon {
  width: 20px;
  height: 20px;
  border-radius: 4px;
  object-fit: contain;
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

/* 占位提示对齐与输入文本对齐：取值来自外观配置写入的 CSS 变量（两者独立控制，
   互不影响：占位提示作用于 naive-ui 的占位符元素，输入文本作用于输入元素本体） */
.search-bar-wrapper :deep(.n-input__placeholder) {
  text-align: var(--search-bar-placeholder-align);
}

/* 占位提示文本块：默认撑满可用宽度；自适应模式收缩到内容宽度并由自动外边距居中
   （内容装不下时宽度回退为可用宽度，文字自左向右溢出、由占位符容器裁切） */
.search-bar-wrapper :deep(.n-input__placeholder > span) {
  display: block;
  width: var(--search-bar-placeholder-span-width);
  max-width: 100%;
  margin: 0 auto;
}

.search-bar-wrapper :deep(.n-input__input-el) {
  text-align: var(--search-bar-input-align);
}
</style>
