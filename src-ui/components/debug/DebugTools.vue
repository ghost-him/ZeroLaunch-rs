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
      />
    </DebugCard>
  </div>
</template>

<script setup lang="ts">
import { h, ref } from 'vue'
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

/**
 * 渲染展开行的分数分解明细，按引擎/增强器产出的顺序保序展示：
 * 乘法项缩进并以 "× 系数" 显示，紧跟在其作用的加分项之后（明细 Vec 保序），
 * 图例行说明乘法语义，使总分可按"加分项累计 × 乘法系数 + 其余加分项"核对。
 */
function renderScoreDetail(details: ScoreDetail[], totalScore: number) {
  return h('div', { class: 'score-detail-expand' }, [
    h(
      'div',
      { class: 'score-detail-header' },
      t('debug.detailExpandTitle', {
        count: details.length,
        score: totalScore.toFixed(4),
      }),
    ),
    h('div', { class: 'score-detail-hint' }, t('debug.detailHint')),
    ...details.map((d) => {
      if (d.kind === 'multiply') {
        return h('div', { class: 'score-detail-row score-detail-multiply' }, [
          h('span', { class: 'score-detail-name' }, d.description),
          h('span', { class: 'score-detail-value' }, `× ${d.score.toFixed(4)}`),
        ])
      }
      return h('div', { class: 'score-detail-row' }, [
        h('span', { class: 'score-detail-name' }, d.description),
        h(
          'span',
          { class: 'score-detail-value' },
          `${d.score.toFixed(4)} × ${d.weight.toFixed(2)} = ${(d.score * d.weight).toFixed(4)}`,
        ),
      ])
    }),
  ])
}

const detailColumns: DataTableColumns<SearchDetailItem> = [
  {
    type: 'expand',
    renderExpand: (row) => renderScoreDetail(row.detailedScore ?? [], row.score),
  },
  { title: t('debug.colRank'), key: 'rank', width: 40 },
  { title: t('debug.colId'), key: 'candidateId', width: 70 },
  { title: t('debug.colName'), key: 'name', width: 140, ellipsis: { tooltip: true } },
  {
    title: t('debug.colScore'),
    key: 'score',
    width: 90,
    render: (row) => row.score.toFixed(4),
  },
  { title: t('debug.colType'), key: 'targetType', width: 80 },
  { title: t('debug.colTarget'), key: 'targetText', ellipsis: { tooltip: true } },
  {
    title: t('debug.colKeywords'),
    key: 'keywords',
    render: (row) => row.keywords.join(', '),
    ellipsis: { tooltip: true },
  },
]
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

.score-detail-expand {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 4px 12px;
}
.score-detail-header {
  margin-bottom: 2px;
  font-weight: 600;
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
}
.score-detail-hint {
  margin-bottom: 4px;
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
}
.score-detail-multiply {
  padding-left: 16px;
  color: var(--text-secondary);
}
.score-detail-row {
  display: flex;
  justify-content: space-between;
  gap: 16px;
  font-size: var(--font-size-sm);
}
.score-detail-name {
  color: var(--text-primary);
}
.score-detail-value {
  font-variant-numeric: tabular-nums;
  color: var(--text-secondary);
}
</style>
