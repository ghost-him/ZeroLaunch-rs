<template>
  <div class="inspector-panel">
    <div class="inspector-header">
      <h3>插件检查器</h3>
      <n-tag v-if="!available" type="warning">
        检查器已禁用 (需启用 cargo feature "inspector")
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
  inspectorSimulateQuery,
} from '@/bridge/commands'
import type { InspectorStateResponse, PluginInspectorInfo, InspectedQueryEvent } from '@/bridge/contract'

const plugins = ref<PluginInspectorInfo[]>([])
const queries = ref<InspectedQueryEvent[]>([])
const totalQueries = ref(0)
const available = ref(true)
const refreshing = ref(false)

const simInput = ref('')
const simResult = ref<string | null>(null)
const simulating = ref(false)

let pollTimer: ReturnType<typeof setInterval> | null = null

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
    const result = await inspectorSimulateQuery(input)
    simResult.value = JSON.stringify(result, null, 2)
  } catch (e) {
    simResult.value = `Error: ${e}`
  } finally {
    simulating.value = false
  }
}

onMounted(async () => {
  await refresh()
  pollTimer = setInterval(refresh, 2000)
})

onUnmounted(() => {
  if (pollTimer) clearInterval(pollTimer)
})
</script>

<style scoped>
.inspector-panel {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 0 4px;
}
.inspector-header {
  display: flex;
  align-items: center;
  gap: 12px;
}
.inspector-header h3 {
  margin: 0;
  font-size: 16px;
}
.inspector-body {
  display: flex;
  gap: 16px;
  flex: 1;
  min-height: 0;
}
.inspector-left {
  flex: 0 0 340px;
  overflow: auto;
}
.inspector-right {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-width: 0;
}
.inspector-section h4 {
  margin: 0 0 6px;
  font-size: 14px;
}
.simulate-row {
  display: flex;
  gap: 8px;
  margin-bottom: 8px;
}
</style>
