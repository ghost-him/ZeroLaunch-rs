<template>
  <div class="param-panel" v-if="store.paramPanelState">
    <div class="param-panel-header">
      <span class="candidate-name">{{ store.paramPanelState.candidateItem.title }}</span>
      <span class="param-hint">请填写参数后按 Enter 执行</span>
    </div>

    <div class="param-fields">
      <div
        v-for="field in store.paramPanelState.fields"
        :key="field.index"
        class="param-field"
      >
        <label>{{ field.label }}</label>
        <n-input
          :ref="(el: any) => setFieldRef(el, field.index)"
          v-model:value="field.value"
          :placeholder="'输入第 ' + (field.index + 1) + ' 个参数'"
          size="large"
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { watch, nextTick, onMounted } from 'vue'
import { NInput } from 'naive-ui'
import { useSearchStore } from '@/stores/search-store'

const store = useSearchStore()

const fieldRefs = new Map<number, InstanceType<typeof NInput>>()

function setFieldRef(el: any, index: number) {
  if (el) {
    fieldRefs.set(index, el)
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
