<template>
  <div class="inspector-panel">
    <div class="inspector-header">
      <h3>插件检查器</h3>
      <n-tag v-if="!available" type="warning">
        检查器已禁用 (需在设置中开启调试模式)
      </n-tag>
      <n-button v-else size="small" @click="refresh" :loading="refreshing">
        刷新
      </n-button>
    </div>

    <template v-if="available">
      <div class="inspector-body">
        <div class="inspector-left">
          <h4>已注册插件 ({{ plugins.length }})</h4>
          <n-data-table
            :columns="pluginColumns"
            :data="plugins"
            :max-height="400"
            :row-key="(row: PluginInspectorInfo) => row.componentId"
            size="small"
            virtual-scroll
          />
        </div>

        <div class="inspector-right">
          <div class="inspector-section">
            <h4>最近查询 ({{ queries.length }} / {{ totalQueries }})</h4>
            <n-data-table
              :columns="queryColumns"
              :data="queries"
              :max-height="250"
              :row-key="(row: InspectedQueryEvent) => row.traceId + row.timestamp"
              size="small"
              virtual-scroll
            />
          </div>

          <div class="inspector-section">
            <h4>手动模拟查询</h4>
            <div class="simulate-row">
              <n-input
                v-model:value="simInput"
                placeholder="输入查询文本..."
                @keyup.enter="simulate"
                clearable
              />
              <n-button
                type="primary"
                size="small"
                @click="simulate"
                :loading="simulating"
              >
                模拟
              </n-button>
            </div>
            <n-code
              v-if="simResult !== null"
              :code="simResult"
              language="json"
              word-wrap
            />
          </div>
        </div>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { NButton, NCode, NDataTable, NInput, NTag, type DataTableColumns } from 'naive-ui'
import {
  inspectorGetState,
  debugSimulateQuery,
} from '@/bridge/commands'
import { onInspectorStateUpdated } from '@/bridge/events'
import type { InspectorStateResponse, PluginInspectorInfo, InspectedQueryEvent } from '@/bridge/contract'
import type { UnlistenFn } from '@tauri-apps/api/event'

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
  { title: 'ID', key: 'componentId', width: 160, ellipsis: { tooltip: true } },
  { title: '名称', key: 'componentName', width: 140 },
  { title: '类型', key: 'componentType', width: 120 },
  {
    title: '状态',
    key: 'enabled',
    width: 60,
    render: (row) => (row.enabled ? '启用' : '禁用'),
  },
]

const queryColumns: DataTableColumns<InspectedQueryEvent> = [
  { title: '时间', key: 'timestamp', width: 160, ellipsis: { tooltip: true } },
  { title: '查询', key: 'rawQuery', width: 160, ellipsis: { tooltip: true } },
  { title: '模式', key: 'mode', width: 90 },
  { title: '结果数', key: 'resultCount', width: 60 },
  { title: '耗时(ms)', key: 'durationMs', width: 70 },
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
  height: 100%;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.inspector-header {
  display: flex;
  align-items: center;
  gap: 8px;
}
.inspector-header h3 {
  margin: 0;
}
.inspector-body {
  display: flex;
  gap: 16px;
  flex: 1;
  overflow: auto;
}
.inspector-left {
  flex: 1;
  min-width: 0;
}
.inspector-right {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.inspector-section {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.inspector-section h4 {
  margin: 0;
}
.simulate-row {
  display: flex;
  gap: 8px;
  align-items: center;
}
.simulate-row .n-input {
  flex: 1;
}
</style>
