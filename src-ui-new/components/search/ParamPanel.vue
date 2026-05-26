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
        <input
          :ref="(el: unknown) => setFieldRef(el as HTMLInputElement | null, field.index)"
          v-model="field.value"
          :placeholder="'输入第 ' + (field.index + 1) + ' 个参数'"
          @keydown.tab.prevent="store.paramPanelFocusNext()"
          @keydown.shift.tab.prevent="store.paramPanelFocusPrev()"
          @keydown.enter.prevent="store.confirmParamPanel()"
          @keydown.escape.prevent="store.exitParamPanelMode()"
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { watch, nextTick } from 'vue'
import { useSearchStore } from '@/stores/search-store'

const store = useSearchStore()

const fieldRefs = new Map<number, HTMLInputElement>()

function setFieldRef(el: HTMLInputElement | null, index: number) {
  if (el) {
    fieldRefs.set(index, el)
  } else {
    fieldRefs.delete(index)
  }
}

// 自动聚焦第一个字段
watch(
  () => store.paramPanelState,
  (state) => {
    if (state) {
      nextTick(() => {
        const firstField = fieldRefs.get(0)
        firstField?.focus()
      })
    }
  },
)

// 聚焦变化时自动 focus 对应字段
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
  gap: 4px;
}

.param-field label {
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
}

.param-field input {
  height: 36px;
  padding: 0 12px;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  background: var(--input-bg);
  color: var(--text-primary);
  font-size: var(--font-size-md);
  outline: none;
  transition: border-color 0.2s;
}

.param-field input:focus {
  border-color: var(--accent-color);
}
</style>
