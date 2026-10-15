<template>
  <div v-if="chooser" class="exclude-panel">
    <div class="exclude-header">
      <span class="exclude-title">{{ t('search.hideResult') }}</span>
      <span class="exclude-target">{{ chooser.item.title }}</span>
    </div>
    <div class="exclude-subtitle">{{ chooser.item.subtitle }}</div>

    <div class="exclude-options">
      <div
        v-for="opt in options"
        :key="opt.kind"
        class="exclude-option"
        :class="{ 'is-selected': chooser.kind === opt.kind, 'is-disabled': !opt.available }"
        @click="onSelectOption(opt)"
      >
        <div class="option-head">
          <span class="option-label">{{ opt.label }}</span>
          <!-- 方式徽标复用设置页规则表的词表（组件文案 components.exclusion-config.options.kind.*），
               便于用户在结果排除表里对上刚写入的规则 -->
          <span class="option-badge">{{ t(`components.exclusion-config.options.kind.${opt.kind}`) }}</span>
        </div>
        <div class="option-desc">{{ opt.desc }}</div>
        <!-- 关键字方式的取值由输入框承担（用户可改写），其余方式直接展示宿主推导的取值 -->
        <div v-if="opt.kind === 'contains'" class="option-input" @click.stop>
          <n-input
            size="small"
            :value="chooser.keyword"
            :disabled="chooser.kind !== 'contains'"
            :placeholder="t('search.excludeKeywordPlaceholder')"
            @update:value="store.setExcludeKeyword"
          />
        </div>
        <div v-else class="option-value">{{ opt.value }}</div>
      </div>
    </div>

    <div class="exclude-hint">{{ t('search.excludeAdvancedHint') }}</div>

    <div class="exclude-actions">
      <n-button size="small" @click="store.closeExcludeChooser()">{{ t('common.cancel') }}</n-button>
      <n-button
        size="small"
        type="primary"
        :disabled="!canApply"
        @click="store.applyExcludeChooser()"
      >
        {{ t('common.confirm') }}
      </n-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { NButton, NInput } from 'naive-ui'
import { useSearchStore } from '@/stores/search-store'
import type { ExclusionKind } from '@/bridge/contract'

const store = useSearchStore()
const { t } = useI18n()

const chooser = computed(() => store.excludeChooser)

/// 选项文案与取值：标签/说明为本组件文案，取值与可用性全部来自宿主推导
/// （folder 为 null 表示该候选项没有文件夹语义，目录方式据此禁用）。
const options = computed(() => {
  const state = chooser.value
  if (!state) return []
  return [
    {
      kind: 'exact' as ExclusionKind,
      available: true,
      label: t('search.excludeExactLabel'),
      desc: t('search.excludeExactDesc'),
      value: state.options.exact,
    },
    {
      kind: 'path_prefix' as ExclusionKind,
      available: state.options.folder !== null,
      label: t('search.excludeFolderLabel'),
      desc: t('search.excludeFolderDesc'),
      value: state.options.folder ?? t('search.excludeFolderUnavailable'),
    },
    {
      kind: 'contains' as ExclusionKind,
      available: true,
      label: t('search.excludeKeywordLabel'),
      desc: t('search.excludeKeywordDesc'),
      value: '',
    },
  ]
})

/// 关键字方式必须有关键字：为空时禁用确认（与 store 的兜底判定同口径）。
const canApply = computed(() => {
  const state = chooser.value
  if (!state) return false
  return state.kind !== 'contains' || state.keyword.trim() !== ''
})

/// 点选隐藏方式：不可用方式（该候选项没有文件夹）不响应点击。
function onSelectOption(opt: { kind: ExclusionKind; available: boolean }) {
  if (!opt.available) return
  store.selectExcludeKind(opt.kind)
}
</script>

<style scoped>
.exclude-panel {
  padding: 14px 24px 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.exclude-header {
  display: flex;
  align-items: baseline;
  gap: 8px;
}

.exclude-title {
  font-size: var(--font-size-base);
  font-weight: 600;
  color: var(--text-primary);
}

.exclude-target {
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
}

.exclude-subtitle {
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
  word-break: break-all;
}

.exclude-options {
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-height: 0;
  overflow-y: auto;
}

.exclude-option {
  padding: 6px 10px;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  cursor: pointer;
}

.exclude-option.is-selected {
  border-color: var(--primary-color);
  background: var(--primary-color-alpha);
}

.exclude-option.is-disabled {
  opacity: 0.55;
  cursor: not-allowed;
}

.option-head {
  display: flex;
  align-items: baseline;
  gap: 8px;
}

.option-label {
  font-size: var(--font-size-sm);
  color: var(--text-primary);
}

.option-badge {
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
}

.option-desc,
.option-value {
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
  word-break: break-all;
  margin-top: 2px;
}

.option-input {
  margin-top: 4px;
}

.exclude-hint {
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
  flex-shrink: 0;
}

.exclude-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  flex-shrink: 0;
}
</style>
