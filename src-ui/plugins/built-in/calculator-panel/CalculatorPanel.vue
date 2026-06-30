<template>
  <div class="calculator-panel">
    <!-- 表达式 / 结果 -->
    <div class="calc-display">
      <div class="calc-expression">{{ expression || '输入表达式...' }}</div>
      <div class="calc-result" :class="{ error: !!error }">
        {{ error ? error : (result ?? '') }}
      </div>
    </div>

    <!-- 动作按钮 -->
    <div class="calc-actions" v-if="actions.length > 0">
      <n-button
        v-for="action in actions"
        :key="action.id"
        size="small"
        :type="action.isDefault ? 'primary' : 'default'"
        @click="executeAction(action)"
      >
        {{ action.label }}
      </n-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { NButton } from 'naive-ui'
import type { ResultAction } from '@/bridge/contract'
import { useSearchStore } from '@/stores/search-store'

const props = defineProps<{
  data: {
    expression?: string
    result?: string | null
    error?: string
    rawValue?: number
  }
  actions: ResultAction[]
}>()

const searchStore = useSearchStore()

const expression = computed(() => props.data?.expression ?? '')
const result = computed(() => props.data?.result ?? null)
const error = computed(() => props.data?.error ?? null)

async function executeAction(action: ResultAction) {
  if (action.id === 'copy_result' && props.data?.result) {
    try {
      await navigator.clipboard.writeText(props.data.result)
    } catch (error) {
      console.warn('[CalculatorPanel] Clipboard access failed:', error)
    }
  }
  // 通知后端执行动作（插件模式下 candidate_id=0 为虚拟值，后端按 plugin_id 路由）
  await searchStore.doConfirm(0, action.id)
}
</script>

<style scoped>
.calculator-panel {
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.calc-display {
  background: var(--bg-secondary);
  border-radius: var(--radius-sm);
  padding: 16px;
  min-height: 60px;
  display: flex;
  flex-direction: column;
  justify-content: center;
  gap: 8px;
}

.calc-expression {
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
  word-break: break-all;
}

.calc-result {
  font-size: 28px;
  font-weight: 600;
  color: var(--text-primary);
  font-variant-numeric: tabular-nums;
}

.calc-result.error {
  font-size: var(--font-size-base);
  color: #d03050;
  font-weight: 400;
}

.calc-actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
}
</style>
