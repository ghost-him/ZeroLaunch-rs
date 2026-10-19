<template>
  <n-tooltip v-if="text" trigger="hover" :style="TOOLTIP_STYLE">
    <template #trigger>
      <!-- 问号只作视觉入口：说明由气泡呈现，读屏不重复播报「?」 -->
      <span class="help-tip" aria-hidden="true">?</span>
    </template>
    {{ text }}
  </n-tooltip>
</template>

<script setup lang="ts">
import { NTooltip } from 'naive-ui'

/**
 * 字段补充说明的问号气泡。
 * 参数：text - 已翻译的说明文本；为空（或 null）时不渲染任何东西。
 */
defineProps<{ text: string | null }>()

/**
 * 气泡容器样式：限宽 360px、按词换行、行高 1.5。
 */
const TOOLTIP_STYLE = {
  maxWidth: '360px',
  whiteSpace: 'normal',
  wordBreak: 'break-word',
  lineHeight: '1.5',
} as const
</script>

<style scoped>
.help-tip {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 16px;
  margin-left: 4px;
  border: 1px solid var(--text-secondary);
  border-radius: 50%;
  color: var(--text-secondary);
  font-size: var(--font-size-sm);
  line-height: 1;
  vertical-align: middle;
  cursor: help;
}
</style>
