<template>
  <div class="debug-tools">
    <!-- 搜索性能测试 -->
    <div class="debug-section">
      <h4 class="section-title">{{ t('debug.searchTime') }}</h4>
      <div class="action-row">
        <n-input
          v-model:value="searchTimeInput"
          :placeholder="t('debug.searchTimePlaceholder')"
          @keyup.enter="runSearchTime"
          clearable
        />
        <n-button type="primary" size="small" :loading="searchTimeLoading" @click="runSearchTime">
          {{ t('debug.run') }}
        </n-button>
      </div>
      <n-descriptions v-if="searchTimeResult" :column="3" size="small" bordered>
        <n-descriptions-item :label="t('debug.duration')">{{ searchTimeResult.durationMs }}ms</n-descriptions-item>
        <n-descriptions-item :label="t('debug.totalCandidates')">{{ searchTimeResult.totalCandidates }}</n-descriptions-item>
        <n-descriptions-item :label="t('debug.resultCount')">{{ searchTimeResult.resultCount }}</n-descriptions-item>
      </n-descriptions>
    </div>

    <!-- 索引性能测试 -->
    <div class="debug-section">
      <h4 class="section-title">{{ t('debug.indexTime') }}</h4>
      <n-button type="primary" size="small" :loading="indexTimeLoading" @click="runIndexTime">
        {{ t('debug.runIndex') }}
      </n-button>
      <n-descriptions v-if="indexTimeResult" :column="2" size="small" bordered>
        <n-descriptions-item :label="t('debug.duration')">{{ indexTimeResult.durationMs }}ms</n-descriptions-item>
        <n-descriptions-item :label="t('debug.totalCandidates')">{{ indexTimeResult.candidateCount }}</n-descriptions-item>
      </n-descriptions>
    </div>

    <!-- 搜索关键字生成 -->
    <div class="debug-section">
      <h4 class="section-title">{{ t('debug.keywordGen') }}</h4>
      <div class="action-row">
        <n-input
          v-model:value="keywordsInput"
          :placeholder="t('debug.namePlaceholder')"
          @keyup.enter="runKeywords"
          clearable
        />
        <n-button type="primary" size="small" :loading="keywordsLoading" @click="runKeywords">
          {{ t('debug.generate') }}
        </n-button>
      </div>
      <div v-if="keywordsResult" class="keywords-result">
        <n-tag v-for="kw in keywordsResult" :key="kw" size="small" style="margin: 2px">
          {{ kw }}
        </n-tag>
      </div>
    </div>

    <!-- 搜索匹配详情 -->
    <div class="debug-section">
      <h4 class="section-title">{{ t('debug.searchDetail') }}</h4>
      <div class="action-row">
        <n-input
          v-model:value="detailInput"
          :placeholder="t('debug.searchTimePlaceholder')"
          @keyup.enter="runDetail"
          clearable
        />
        <n-button type="primary" size="small" :loading="detailLoading" @click="runDetail">
          {{ t('debug.search') }}
        </n-button>
      </div>
      <n-data-table
        v-if="detailResult"
        :columns="detailColumns"
        :data="detailResult"
        size="small"
        :max-height="300"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { h, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  NButton, NDataTable, NDescriptions, NDescriptionsItem,
  NInput, NTag,
  type DataTableColumns,
} from 'naive-ui'
import {
  debugTestSearchTime,
  debugTestIndexTime,
  debugGetSearchKeys,
  debugSearchDetail,
} from '@/bridge/commands'
import type {
  SearchTimingResult,
  IndexTimingResult,
  SearchDetailItem,
  ScoreDetail,
} from '@/bridge/contract'

const { t } = useI18n()

// ---- 搜索性能测试 ----
const searchTimeInput = ref('')
const searchTimeLoading = ref(false)
const searchTimeResult = ref<SearchTimingResult | null>(null)

async function runSearchTime() {
  const input = searchTimeInput.value.trim()
  if (!input) return
  searchTimeLoading.value = true
  try {
    searchTimeResult.value = await debugTestSearchTime(input)
  } catch (e) {
    console.error('[Debug] 搜索性能测试失败:', e)
  } finally {
    searchTimeLoading.value = false
  }
}

// ---- 索引性能测试 ----
const indexTimeLoading = ref(false)
const indexTimeResult = ref<IndexTimingResult | null>(null)

async function runIndexTime() {
  indexTimeLoading.value = true
  try {
    indexTimeResult.value = await debugTestIndexTime()
  } catch (e) {
    console.error('[Debug] 索引性能测试失败:', e)
  } finally {
    indexTimeLoading.value = false
  }
}

// ---- 搜索关键字生成 ----
const keywordsInput = ref('')
const keywordsLoading = ref(false)
const keywordsResult = ref<string[] | null>(null)

async function runKeywords() {
  const input = keywordsInput.value.trim()
  if (!input) return
  keywordsLoading.value = true
  try {
    keywordsResult.value = await debugGetSearchKeys(input)
  } catch (e) {
    console.error('[Debug] 关键字生成失败:', e)
  } finally {
    keywordsLoading.value = false
  }
}

// ---- 搜索匹配详情 ----
const detailInput = ref('')
const detailLoading = ref(false)
const detailResult = ref<SearchDetailItem[] | null>(null)

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

async function runDetail() {
  const input = detailInput.value.trim()
  if (!input) return
  detailLoading.value = true
  try {
    detailResult.value = await debugSearchDetail(input)
  } catch (e) {
    console.error('[Debug] 搜索详情失败:', e)
  } finally {
    detailLoading.value = false
  }
}
</script>

<style scoped>
.debug-tools {
  display: flex;
  flex-direction: column;
  gap: 24px;
}
.debug-section {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.section-title {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
  border-bottom: 1px solid var(--border-color);
  padding-bottom: 4px;
}
.action-row {
  display: flex;
  gap: 8px;
  align-items: center;
}
.action-row .n-input {
  flex: 1;
}
.keywords-result {
  display: flex;
  flex-wrap: wrap;
  gap: 2px;
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
