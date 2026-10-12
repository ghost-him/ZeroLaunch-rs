<template>
  <component
    :is="customRenderer"
    v-if="customRenderer"
    :item="item"
    :selected="selected"
    :index="index"
  />
  <div
    v-else
    class="result-item"
    :class="{ selected }"
    @click="$emit('confirm')"
    @contextmenu.prevent="onContextMenu"
  >
    <div class="item-icon">
      <IconDisplay :src="item.icon" :size="iconSize" />
    </div>
    <div class="item-text">
      <div class="item-title">{{ item.title }}</div>
      <n-tooltip
        v-if="showSubtitle && item.subtitle"
        :disabled="!subtitleTruncated"
        :delay="300"
        placement="bottom-start"
        :style="subtitleTooltipStyle"
      >
        <template #trigger>
          <div class="item-subtitle" @mouseenter="measureSubtitle">{{ subtitleText }}</div>
        </template>
        {{ subtitleText }}
      </n-tooltip>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import type { Component, CSSProperties } from 'vue'
import { NTooltip } from 'naive-ui'
import { resolveText } from '../../i18n'
import type { ListItem } from '../../bridge/contract'
import { usePluginStore } from '../../stores/plugin-store'
import IconDisplay from '../common/IconDisplay.vue'
import type { CtxItem } from '../layout/ContextMenu.vue'

const iconSize = computed(() => {
  const raw = getComputedStyle(document.documentElement).getPropertyValue('--result-item-icon-size').trim()
  const num = parseFloat(raw)
  return isNaN(num) ? 32 : num
})

const showSubtitle = computed(() => {
  const raw = getComputedStyle(document.documentElement).getPropertyValue('--show-subtitle').trim()
  return raw !== '0'
})

const props = defineProps<{
  item: ListItem
  selected: boolean
  index: number
}>()

const emit = defineEmits<{
  (e: 'confirm'): void
  (e: 'context-action', actionId: string): void
  (e: 'contextmenu', x: number, y: number, items: CtxItem[]): void
}>()

const pluginStore = usePluginStore()

const customRenderer = computed<Component | null>(() =>
  pluginStore.getResultItemComponent(props.item.targetType),
)

/// 副标题完整文本（路径/URL 原文或插件描述），由后端下发，此处不截断
const subtitleText = computed(() => resolveText(props.item.subtitle))

/// 副标题是否被容器截断：仅截断时才弹提示，完整可见的短文本不打扰
const subtitleTruncated = ref(false)

/// 提示内容样式：按字符换行；宽度留出条目左侧图标与两侧留白，
/// 保证 vueuc 定位器无论选 start/center/end 对齐都能整体落在窗口内
const subtitleTooltipStyle: CSSProperties = {
  maxWidth: 'calc(100vw - 120px)',
  wordBreak: 'break-all',
  whiteSpace: 'normal',
}

/// 悬停时测量副标题实际宽度与可视宽度，判定是否需要展示完整内容
function measureSubtitle(e: MouseEvent) {
  const el = e.currentTarget as HTMLElement
  subtitleTruncated.value = el.scrollWidth > el.clientWidth
}

// 列表按 id 复用组件实例，同一实例可能承载不同查询的副标题；
// 文本变化后旧判定失效，重置为不弹提示，等下次悬停重新测量
watch(subtitleText, () => {
  subtitleTruncated.value = false
})

function onContextMenu(e: MouseEvent) {
  const items: CtxItem[] = props.item.actions.map((a) => ({
    key: a.id,
    label: resolveText(a.label),
    action: () => emit('context-action', a.id),
  }))
  emit('contextmenu', e.clientX, e.clientY, items)
}
</script>

<style scoped>
.result-item {
  height: var(--result-item-height);
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 16px; /* Increased gap for breathing room */
  padding: 8px 16px; /* Larger padding */
  margin-bottom: var(--result-item-gap); /* Slight separation */
  border-radius: var(--radius-md); /* Softer corners */
  cursor: pointer;
  transition: all 0.2s ease; /* Smoother transition */
  border: 1px solid transparent; /* Prepare for possible active state without shifting layout */
}

.result-item:hover,
.result-item.selected {
  background-color: var(--hover-color); /* Use subtle hover color rather than strong secondary background */
}

.item-icon {
  flex-shrink: 0;
  width: var(--result-item-icon-size);
  height: var(--result-item-icon-size);
  display: flex;
  align-items: center;
  justify-content: center;
}

.item-text {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  justify-content: center;
}

.item-title {
  font-size: var(--font-size-lg);
  font-family: var(--result-item-font-family);
  font-weight: 500;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  letter-spacing: 0.2px;
}

.item-subtitle {
  font-size: var(--font-size-md);
  font-family: var(--result-item-font-family);
  color: var(--text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  margin-top: 2px;
}
</style>
