<template>
  <div class="debug-tools">
    <DebugCard :title="t('debug.searchTime')" :description="t('debug.descSearchTime')" class="wide">
      <div class="tool-row">
        <n-input
          v-model:value="searchTime.input"
          :placeholder="t('debug.searchTimePlaceholder')"
          clearable
          @keyup.enter="searchTime.run"
        />
        <n-button type="primary" size="small" :loading="searchTime.loading" @click="searchTime.run">
          {{ t('debug.run') }}
        </n-button>
      </div>
      <div v-if="searchTime.result" class="stat-row">
        <n-statistic :label="t('debug.duration')">{{ searchTime.result.durationMs }} ms</n-statistic>
        <n-statistic :label="t('debug.totalCandidates')">{{ searchTime.result.totalCandidates }}</n-statistic>
        <n-statistic :label="t('debug.resultCount')">{{ searchTime.result.resultCount }}</n-statistic>
      </div>
    </DebugCard>

    <DebugCard :title="t('debug.indexTime')" :description="t('debug.descIndexTime')">
      <div class="tool-row">
        <n-button type="primary" size="small" :loading="indexLoading" @click="runIndexTime">
          {{ t('debug.runIndex') }}
        </n-button>
      </div>
      <div v-if="indexResult" class="stat-row">
        <n-statistic :label="t('debug.duration')">{{ indexResult.durationMs }} ms</n-statistic>
        <n-statistic :label="t('debug.totalCandidates')">{{ indexResult.candidateCount }}</n-statistic>
      </div>
    </DebugCard>

    <DebugCard :title="t('debug.keywordGen')" :description="t('debug.descKeywordGen')">
      <div class="tool-row">
        <n-input
          v-model:value="keywords.input"
          :placeholder="t('debug.namePlaceholder')"
          clearable
          @keyup.enter="keywords.run"
        />
        <n-button type="primary" size="small" :loading="keywords.loading" @click="keywords.run">
          {{ t('debug.generate') }}
        </n-button>
      </div>
      <div v-if="keywords.result" class="tag-list">
        <n-tag v-for="kw in keywords.result" :key="kw" size="small">
          {{ kw }}
        </n-tag>
      </div>
    </DebugCard>

    <DebugCard :title="t('debug.searchDetail')" :description="t('debug.descSearchDetail')" class="wide">
      <div class="tool-row">
        <n-input
          v-model:value="detail.input"
          :placeholder="t('debug.searchTimePlaceholder')"
          clearable
          @keyup.enter="detail.run"
        />
        <n-button type="primary" size="small" :loading="detail.loading" @click="detail.run">
          {{ t('debug.search') }}
        </n-button>
        <n-button size="small" :disabled="!detail.result?.length" @click="copyDetail">
          {{ t('debug.copy') }}
        </n-button>
      </div>
      <n-data-table
        v-if="detail.result"
        :columns="detailColumns"
        :data="detail.result"
        size="small"
        :max-height="420"
        :scroll-x="tableScrollX"
      />
    </DebugCard>

    <DebugCard
      :title="t('debug.agentTrace')"
      :description="t('debug.descAgentTrace')"
      class="wide"
    >
      <template #actions>
        <n-button size="small" :loading="agentLoading" @click="loadTraces">
          {{ t('debug.refresh') }}
        </n-button>
      </template>
      <n-empty v-if="!agentTraces.length" :description="t('debug.agentEmpty')" size="small" />
      <n-collapse v-else :default-expanded-names="[0]">
        <n-collapse-item v-for="(trace, index) in agentTraces" :key="index" :name="index">
          <template #header>{{ traceTitle(trace) }}</template>
          <div class="trace-body">
            <div class="trace-meta">{{ trace.time }} · {{ trace.modelId }}</div>
            <div v-if="trace.error" class="trace-error">
              {{ t('debug.agentError') }}: {{ trace.error }}
            </div>
            <div class="trace-section">
              <div class="trace-label">{{ t('debug.agentContext') }}</div>
              <div v-for="(message, mIndex) in trace.messages" :key="mIndex" class="trace-message">
                <span class="trace-role">{{ message.role }}</span>
                <pre class="trace-pre">{{ message.content }}</pre>
              </div>
            </div>
            <div class="trace-section">
              <div class="trace-label">{{ t('debug.agentTools') }}</div>
              <div class="trace-tools">{{ trace.tools.join(', ') }}</div>
            </div>
            <div v-for="(round, rIndex) in trace.rounds" :key="rIndex" class="trace-section">
              <div class="trace-label">{{ t('debug.agentRound', { n: rIndex + 1 }) }}</div>
              <pre v-if="round.content" class="trace-pre">{{ round.content }}</pre>
              <div v-for="(call, cIndex) in round.toolCalls" :key="cIndex" class="trace-call">
                <div class="trace-label">{{ t('debug.agentToolCall', { name: call.name }) }}</div>
                <pre class="trace-pre">{{ call.arguments }}</pre>
                <div class="trace-label">{{ t('debug.agentToolResult') }}</div>
                <pre class="trace-pre">{{ call.result }}</pre>
              </div>
            </div>
          </div>
        </n-collapse-item>
      </n-collapse>
    </DebugCard>
  </div>
</template>

<script setup lang="ts">
import { computed, h, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  NButton, NCollapse, NCollapseItem, NDataTable, NEmpty, NInput, NStatistic, NTag,
  useMessage,
  type DataTableColumns,
} from 'naive-ui'
import DebugCard from './DebugCard.vue'
import { useQueryTool } from '../../composables/useQueryTool'
import {
  debugTestSearchTime,
  debugTestIndexTime,
  debugGetSearchKeys,
  debugSearchDetail,
  debugCopySearchDetail,
  debugGetAgentTraces,
} from '@/bridge/commands'
import type {
  AgentTrace,
  IndexTimingResult,
  SearchDetailItem,
  ScoreDetail,
} from '@/bridge/contract'

const { t } = useI18n()
const message = useMessage()

// ---- 查询型工具：输入 + 执行 + 结果由 useQueryTool 统一管理 ----
const searchTime = useQueryTool(debugTestSearchTime)
const keywords = useQueryTool(debugGetSearchKeys)
const detail = useQueryTool(debugSearchDetail)

// ---- AI 助手轨迹（无输入，刷新按钮拉取最近若干次提问） ----
const agentTraces = ref<AgentTrace[]>([])
const agentLoading = ref(false)

// ---- 索引性能测试（无输入，独立状态） ----
const indexLoading = ref(false)
const indexResult = ref<IndexTimingResult | null>(null)

async function runIndexTime() {
  indexLoading.value = true
  try {
    indexResult.value = await debugTestIndexTime()
  } catch {
    message.error(t('debug.queryFailed'))
  } finally {
    indexLoading.value = false
  }
}

/** 拉取最近若干次 AI 助手提问的调试轨迹（调试模式未开启时后端返回错误）。 */
async function loadTraces() {
  agentLoading.value = true
  try {
    agentTraces.value = await debugGetAgentTraces()
  } catch {
    message.error(t('debug.queryFailed'))
  } finally {
    agentLoading.value = false
  }
}

/** 轨迹标题：取最后一条用户消息（本次提问）；没有提问时退回记录时间。 */
function traceTitle(trace: AgentTrace): string {
  const question = [...trace.messages].reverse().find((message) => message.role === 'user')?.content
  return question ? question.slice(0, 80) : trace.time
}

onMounted(loadTraces)

/** 固定列基础宽度之和（含总分列），用于计算表格横向滚动宽度。 */
const BASE_COLUMN_WIDTH = 40 + 70 + 160 + 80 + 180 + 160 + 90
/** 单个分数明细列宽度。 */
const SCORE_COLUMN_WIDTH = 120

/**
 * 分数明细列：取结果集中出现的全部明细项（按首次出现顺序去重）生成动态列，
 * 复刻旧版调试页"每项分数一列、末尾总分"的平铺展示。
 * - add 项：主值 = 未加权分值，副文本 = × 权重（与旧版 历史分 (x权重) 一致）；
 * - multiply 项：显示 × 系数，传达乘法语义。
 */
const scoreColumns = computed<DataTableColumns<SearchDetailItem>>(() => {
  const rows = detail.result ?? []
  const seen = new Set<string>()
  const items: ScoreDetail[] = []
  for (const row of rows) {
    for (const d of row.detailedScore ?? []) {
      if (!seen.has(d.description)) {
        seen.add(d.description)
        items.push(d)
      }
    }
  }
  return items.map((it) => ({
    title: it.description,
    key: it.description,
    width: SCORE_COLUMN_WIDTH,
    render: (row: SearchDetailItem) => {
      const d = row.detailedScore?.find((x) => x.description === it.description)
      if (!d) return '—'
      if (d.kind === 'multiply') {
        return h('span', { class: 'score-cell score-cell-multiply' }, `× ${d.score.toFixed(4)}`)
      }
      return h('div', { class: 'score-cell' }, [
        h('div', { class: 'score-cell-value' }, d.score.toFixed(4)),
        h('div', { class: 'score-cell-sub' }, t('debug.scoreWeight', { weight: d.weight.toFixed(2) })),
      ])
    },
  }))
})

const tableScrollX = computed(() => BASE_COLUMN_WIDTH + scoreColumns.value.length * SCORE_COLUMN_WIDTH)

const detailColumns = computed<DataTableColumns<SearchDetailItem>>(() => [
  { title: t('debug.colRank'), key: 'rank', width: 40 },
  { title: t('debug.colId'), key: 'candidateId', width: 70 },
  { title: t('debug.colName'), key: 'name', width: 160, ellipsis: { tooltip: true } },
  { title: t('debug.colType'), key: 'targetType', width: 80 },
  { title: t('debug.colTarget'), key: 'targetText', width: 180, ellipsis: { tooltip: true } },
  {
    title: t('debug.colKeywords'),
    key: 'keywords',
    width: 160,
    render: (row) => row.keywords.join(', '),
    ellipsis: { tooltip: true },
  },
  ...scoreColumns.value,
  {
    title: t('debug.colScore'),
    key: 'score',
    width: 90,
    render: (row) => h('strong', { class: 'score-cell-total' }, row.score.toFixed(4)),
  },
])
/**
 * 按展示列序（固定列 + 动态分数列 + 总分）将详情结果组装为制表符分隔文本。
 * 取前 15 条；表头为各列标题。供复制按钮写入剪贴板。
 */
function buildDetailText(): string {
  const rows = detail.result ?? []
  const header = [
    t('debug.colRank'),
    t('debug.colId'),
    t('debug.colName'),
    t('debug.colType'),
    t('debug.colTarget'),
    t('debug.colKeywords'),
    ...scoreColumns.value.map((c) => String((c as { title?: unknown }).title ?? '')),
    t('debug.colScore'),
  ]
  const cells = (row: SearchDetailItem): string[] => [
    String(row.rank),
    String(row.candidateId),
    row.name,
    row.targetType,
    row.targetText,
    row.keywords.join(', '),
    ...scoreColumns.value.map((c) => {
      const key = (c as { key: string }).key
      const d = row.detailedScore?.find((x) => x.description === key)
      if (!d) return '—'
      return d.kind === 'multiply' ? `× ${d.score.toFixed(4)}` : `${d.score.toFixed(4)} (×${d.weight.toFixed(2)})`
    }),
    row.score.toFixed(4),
  ]
  const lines = [header.join('\t'), ...rows.slice(0, 15).map((r) => cells(r).join('\t'))]
  return lines.join('\n')
}

/** 复制前 15 条详情到剪贴板（经后端写入，符合前后端职责边界）。 */
async function copyDetail() {
  if (!detail.result?.length) {
    message.warning(t('debug.noResult'))
    return
  }
  try {
    await debugCopySearchDetail(buildDetailText())
    message.success(t('debug.copied'))
  } catch {
    message.error(t('debug.queryFailed'))
  }
}
</script>

<style scoped>
.debug-tools {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
  padding-bottom: 16px;
}

.wide {
  grid-column: 1 / -1;
}

.tool-row {
  display: flex;
  gap: 8px;
  align-items: center;
  flex-shrink: 0;
}

.tool-row .n-input {
  flex: 1;
}

.stat-row {
  display: flex;
  gap: 32px;
  align-items: flex-start;
  flex-shrink: 0;
}

.tag-list {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.score-cell {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.score-cell-value {
  font-variant-numeric: tabular-nums;
  color: var(--text-primary);
}
.score-cell-sub {
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
}
.score-cell-multiply {
  font-variant-numeric: tabular-nums;
  color: var(--text-secondary);
}
.score-cell-total {
  font-variant-numeric: tabular-nums;
  color: var(--text-primary);
}

/* 轨迹文本可选中复制（覆盖全局 body 的 user-select: none） */
.trace-body {
  user-select: text;
  -webkit-user-select: text;
}

.trace-meta {
  margin-bottom: 8px;
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
}

.trace-error {
  margin-bottom: 8px;
  font-size: var(--font-size-sm);
  color: var(--text-error);
}

.trace-section {
  margin-bottom: 12px;
}

.trace-label {
  margin-bottom: 4px;
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
}

.trace-role {
  display: inline-block;
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
}

.trace-tools {
  font-size: var(--font-size-sm);
  font-family: monospace;
}

.trace-pre {
  margin: 2px 0 0;
  padding: 6px 8px;
  max-height: 240px;
  overflow: auto;
  font-size: var(--font-size-sm);
  white-space: pre-wrap;
  word-break: break-word;
  background: var(--bg-color);
  border-radius: 4px;
}
</style>
