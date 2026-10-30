<template>
  <div class="param-panel" v-if="store.paramPanelState">
    <div class="param-panel-header">
      <span class="candidate-name">{{ store.paramPanelState.candidateItem.title }}</span>
      <span class="param-hint" :class="{ 'param-hint-invalid': hintInvalid }">{{ hintText }}</span>
    </div>

    <div class="param-fields">
      <div
        v-for="field in store.paramPanelState.fields"
        :key="field.index"
        class="param-field"
      >
        <label>{{ field.label }}</label>
        <n-input
          :ref="(el) => setFieldRef(el, field.index)"
          v-model:value="field.value"
          :placeholder="$t('search.paramPlaceholder', { n: field.index + 1 })"
          size="large"
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { watch, nextTick, onMounted, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { NInput } from 'naive-ui'
import { useSearchStore } from '@/stores/search-store'

const store = useSearchStore()
const { t } = useI18n()

/// 面板仍有未填字段。
const hasEmptyField = computed(
  () => store.paramPanelState?.fields.some((f) => f.value.trim() === '') ?? false,
)

/// 提交被拒后的报红态：只在仍缺参数时成立，补齐即自动消失。
const hintInvalid = computed(() => !!store.paramPanelState?.invalid && hasEmptyField.value)

/// 提示文案与真实按键行为一致：多字段时 Enter 逐项前进，末项才执行。
const hintText = computed(() => {
  const state = store.paramPanelState
  if (!state) return ''
  if (hintInvalid.value) return t('search.paramIncomplete')
  return state.fields.length > 1 ? t('search.paramHintMulti') : t('search.paramHint')
})

const fieldRefs = new Map<number, InstanceType<typeof NInput>>()

function setFieldRef(el: unknown, index: number) {
  if (el) {
    fieldRefs.set(index, el as InstanceType<typeof NInput>)
  } else {
    fieldRefs.delete(index)
  }
}

// 进入面板时自动聚焦第一个字段。
// 不能放在 setFieldRef 中，因为内联 :ref 回调在每次组件重渲染时都会被 Vue 重新调用
//（新旧函数引用不同），导致打字时焦点被错误地抢回第一个字段。
// onMounted 只在首次挂载时执行一次，不受后续 v-model 触发的重渲染影响。
onMounted(() => {
  nextTick(() => {
    const el = fieldRefs.get(store.paramPanelState?.focusedFieldIndex ?? 0)
    el?.focus()
  })
})

// Tab 切换字段时重新聚焦（此时元素已在 DOM 中，单次 nextTick 足够）
watch(
  () => store.paramPanelState?.focusedFieldIndex,
  (idx) => {
    if (idx !== undefined) {
      nextTick(() => {
        const el = fieldRefs.get(idx)
        el?.focus()
      })
    }
  },
)
</script>

<style scoped>
.param-panel {
  padding: 16px 24px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.param-panel-header {
  display: flex;
  align-items: center;
  gap: 12px;
}

.candidate-name {
  font-size: var(--font-size-lg);
  font-weight: 600;
  color: var(--text-primary);
}

.param-hint {
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
}

.param-hint-invalid {
  color: var(--text-error);
}

.param-fields {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.param-field {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.param-field label {
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
  padding-left: 2px;
}
</style>
