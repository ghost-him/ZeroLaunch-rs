<template>
  <div class="inspector-panel">
    <n-alert v-if="!available" type="warning" :title="t('inspector.disabledTag')" />

    <template v-else>
      <DebugCard :title="t('inspector.pluginsTitle', { count: plugins.length })">
        <template #actions>
          <n-button size="small" :loading="refreshing" @click="refresh">
            {{ t('inspector.refresh') }}
          </n-button>
        </template>
        <n-data-table
          :columns="pluginColumns"
          :data="plugins"
          :max-height="300"
          :row-key="(row: PluginInspectorInfo) => row.componentId"
          :empty-text="t('inspector.emptyPlugins')"
          size="small"
          virtual-scroll
        />
      </DebugCard>

      <DebugCard :title="t('inspector.queriesTitle', { count: queries.length, total: totalQueries })">
        <n-data-table
          :columns="queryColumns"
          :data="queries"
          :max-height="240"
          :row-key="(row: InspectedQueryEvent) => row.traceId + row.timestamp"
          :empty-text="t('inspector.emptyQueries')"
          size="small"
          virtual-scroll
        />
      </DebugCard>

      <DebugCard :title="t('inspector.simulateTitle')">
        <div class="simulate-row">
          <n-input
            v-model:value="simInput"
            :placeholder="t('inspector.simulatePlaceholder')"
            clearable
            @keyup.enter="simulate"
          />
          <n-button type="primary" size="small" :loading="simulating" @click="simulate">
            {{ t('inspector.simulate') }}
          </n-button>
        </div>
        <n-code
          v-if="simResult !== null"
          :code="simResult"
          language="json"
          word-wrap
          class="sim-result"
        />
      </DebugCard>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, h, onMounted, onUnmounted } from 'vue'
import {
  NAlert, NButton, NCode, NDataTable, NInput, NTag,
  type DataTableColumns,
} from 'naive-ui'
import { useI18n } from 'vue-i18n'
import DebugCard from './DebugCard.vue'
import {
  inspectorGetState,
  debugSimulateQuery,
} from '@/bridge/commands'
import { onInspectorStateUpdated } from '@/bridge/events'
import type {
  InspectorStateResponse,
  PluginInspectorInfo,
  InspectedQueryEvent,
} from '@/bridge/contract'
import type { UnlistenFn } from '@tauri-apps/api/event'

const { t } = useI18n()

const plugins = ref<PluginInspectorInfo[]>([])
const queries = ref<InspectedQueryEvent[]>([])
const totalQueries = ref(0)
const available = ref(true)
const refreshing = ref(false)

const simInput = ref('')
const simResult = ref<string | null>(null)
const simulating = ref(false)

let unlistenInspector: UnlistenFn | null = null

const pluginColumns: DataTableColumns<PluginInspectorInfo> = [
  { title: t('inspector.colId'), key: 'componentId', width: 180, ellipsis: { tooltip: true } },
  { title: t('inspector.colName'), key: 'componentName', width: 160 },
  { title: t('inspector.colType'), key: 'componentType', width: 130 },
  {
    title: t('inspector.colStatus'),
    key: 'enabled',
    width: 70,
    render: (row) => (
      h(NTag, { size: 'small', type: row.enabled ? 'success' : 'default', bordered: false },
        () => row.enabled ? t('inspector.enabled') : t('inspector.disabled'))
    ),
  },
]

const queryColumns: DataTableColumns<InspectedQueryEvent> = [
  { title: t('inspector.time'), key: 'timestamp', width: 160, ellipsis: { tooltip: true } },
  { title: t('inspector.query'), key: 'rawQuery', width: 180, ellipsis: { tooltip: true } },
  { title: t('inspector.mode'), key: 'mode', width: 90 },
  { title: t('inspector.resultCount'), key: 'resultCount', width: 70 },
  { title: t('inspector.durationMs'), key: 'durationMs', width: 90 },
  { title: t('inspector.owner'), key: 'ownerId', width: 120 },
]

async function refresh() {
  refreshing.value = true
  try {
    const state: InspectorStateResponse = await inspectorGetState()
    if (state.available === false) {
      available.value = false
      return
    }
    available.value = true
    // 仅数据变化时才更新 ref，避免 Vue 不必要的重新渲染
    if (state.totalQueriesLogged !== undefined && state.totalQueriesLogged !== totalQueries.value) {
      queries.value = state.recentQueries ?? []
      totalQueries.value = state.totalQueriesLogged
    }
    if (state.registeredPlugins !== undefined) {
      plugins.value = state.registeredPlugins
    }
  } catch (e) {
    console.error('[Inspector] 获取状态失败:', e)
  } finally {
    refreshing.value = false
  }
}

async function simulate() {
  const input = simInput.value.trim()
  if (!input) return
  simulating.value = true
  try {
    const result = await debugSimulateQuery(input)
    simResult.value = JSON.stringify(result, null, 2)
  } catch (e) {
    simResult.value = `Error: ${e}`
  } finally {
    simulating.value = false
  }
}

onMounted(async () => {
  await refresh()
  unlistenInspector = await onInspectorStateUpdated(() => {
    refresh()
  })
})

onUnmounted(() => {
  unlistenInspector?.()
})
</script>

<style scoped>
.inspector-panel {
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding-bottom: 16px;
}

.simulate-row {
  display: flex;
  gap: 8px;
  align-items: center;
  flex-shrink: 0;
}

.simulate-row .n-input {
  flex: 1;
}

.sim-result {
  max-height: 320px;
  overflow: auto;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  padding: 8px;
}
</style>
