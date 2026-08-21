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
  </div>
</template>

<script setup lang="ts">
import { computed, h, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  NButton, NDataTable, NInput, NStatistic, NTag,
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
} from '@/bridge/commands'
import type {
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
</style>
