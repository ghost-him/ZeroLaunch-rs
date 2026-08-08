<template>
  <div ref="listRef" class="result-list" data-no-drag>
    <ResultItem
      v-for="(item, index) in results"
      :key="item.id"
      :item="item"
      :selected="index === selectedIndex"
      :index="index"
      @confirm="handleItemConfirm(index)"
      @context-action="(actionId: string) => emit('context-action', index, actionId)"
      @contextmenu="(x: number, y: number, items: CtxItem[]) => emit('contextmenu', x, y, items)"
    />
    <div v-if="results.length === 0" class="no-results">
      无结果
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import ResultItem from './ResultItem.vue'
import type { ListItem } from '../../bridge/contract'
import type { CtxItem } from '../layout/ContextMenu.vue'

const props = defineProps<{
  results: ListItem[]
  selectedIndex: number
}>()

const emit = defineEmits<{
  (e: 'select', index: number): void
  (e: 'confirm', index: number, actionIdx?: number): void
  (e: 'context-action', index: number, actionId: string): void
  (e: 'contextmenu', x: number, y: number, items: CtxItem[]): void
}>()

const listRef = ref<HTMLElement | null>(null)

/// 选中项变化（键盘循环导航/新查询重置）时滚动到可见区域。
/// 布局契约：容器无纵向 padding（留白在 margin，滚动盒之外），条目占位 = pitch 恒定，
/// clientHeight = N×pitch 精确成立 → scrollIntoView('nearest') 的贴边目标天然落在条目网格上：
/// 贴顶 = idx×pitch（精确对齐）；贴底 = 选中项底贴视口底，视口内恰好 N 条完整，
/// 残差（gap 高）落在条目透明 margin 区，视觉无残影。无需手动计算滚动位置。
/// 越界语义对齐老版 scrollToSelectedItem：选中项移出可视区才滚动。
/// 定位用 children[idx]（默认渲染器根即 .result-item；自定义渲染器为多根 fragment 时索引会偏移，
/// 当前无插件注册自定义结果渲染器）。
watch(
  [() => props.results, () => props.selectedIndex],
  () => {
    const container = listRef.value
    if (!container || props.results.length === 0) return
    container.children[props.selectedIndex]?.scrollIntoView({ block: 'nearest', behavior: 'smooth' })
  },
  { flush: 'post' },
)

function handleItemConfirm(index: number) {
  // 单机执行：先更新选中状态，再触发执行
  emit('select', index)
  emit('confirm', index)
}
</script>

<style scoped>
.result-list {
  --rl-margin-y: 8px;
  flex: 1;
  /* 上下留白放 margin（滚动盒之外），不参与滚动几何：容器无纵向 padding →
     内容坐标原点 = 容器顶、条目 i 顶 = i × pitch、clientHeight = N × pitch 精确成立，
     scrollIntoView 贴边目标天然落在条目网格上 */
  margin: var(--rl-margin-y) 0;
  min-height: calc(var(--result-item-pitch) * 1);
  max-height: calc(var(--result-item-pitch) * var(--max-visible-results));
  overflow-y: auto;
  padding: 0 12px;
}

/* Hide scrollbar for a cleaner look, similar to modern OS / sofast */
.result-list::-webkit-scrollbar {
  width: 4px;
}
.result-list::-webkit-scrollbar-thumb {
  background: rgba(0, 0, 0, 0.1);
  border-radius: 4px;
}
html.dark .result-list::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.1);
}

.no-results {
  padding: 32px 24px; /* More padding for empty state */
  text-align: center;
  color: var(--text-secondary);
  font-size: var(--font-size-base);
  opacity: 0.8;
}
</style>
