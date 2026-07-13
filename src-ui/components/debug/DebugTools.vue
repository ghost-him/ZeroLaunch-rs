<template>
  <div class="debug-tools">
    <!-- 搜索性能测试 -->
    <div class="debug-section">
      <h4 class="section-title">搜索性能测试</h4>
      <div class="action-row">
        <n-input
          v-model:value="searchTimeInput"
          placeholder="输入查询词..."
          @keyup.enter="runSearchTime"
          clearable
        />
        <n-button type="primary" size="small" :loading="searchTimeLoading" @click="runSearchTime">
          运行
        </n-button>
      </div>
      <n-descriptions v-if="searchTimeResult" :column="3" size="small" bordered>
        <n-descriptions-item label="耗时">{{ searchTimeResult.durationMs }}ms</n-descriptions-item>
        <n-descriptions-item label="候选总数">{{ searchTimeResult.totalCandidates }}</n-descriptions-item>
        <n-descriptions-item label="结果数">{{ searchTimeResult.resultCount }}</n-descriptions-item>
      </n-descriptions>
    </div>

    <!-- 索引性能测试 -->
    <div class="debug-section">
      <h4 class="section-title">索引性能测试</h4>
      <n-button type="primary" size="small" :loading="indexTimeLoading" @click="runIndexTime">
        运行索引
      </n-button>
      <n-descriptions v-if="indexTimeResult" :column="2" size="small" bordered>
        <n-descriptions-item label="耗时">{{ indexTimeResult.durationMs }}ms</n-descriptions-item>
        <n-descriptions-item label="候选总数">{{ indexTimeResult.candidateCount }}</n-descriptions-item>
      </n-descriptions>
    </div>

    <!-- 搜索关键字生成 -->
    <div class="debug-section">
      <h4 class="section-title">搜索关键字生成</h4>
      <div class="action-row">
        <n-input
          v-model:value="keywordsInput"
          placeholder="输入程序名称..."
          @keyup.enter="runKeywords"
          clearable
        />
        <n-button type="primary" size="small" :loading="keywordsLoading" @click="runKeywords">
          生成
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
      <h4 class="section-title">搜索匹配详情</h4>
      <div class="action-row">
        <n-input
          v-model:value="detailInput"
          placeholder="输入查询词..."
          @keyup.enter="runDetail"
          clearable
        />
        <n-button type="primary" size="small" :loading="detailLoading" @click="runDetail">
          搜索
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
import { ref } from 'vue'
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
} from '@/bridge/contract'

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

const detailColumns: DataTableColumns<SearchDetailItem> = [
  { title: '#', key: 'rank', width: 40 },
  { title: '名称', key: 'name', width: 160, ellipsis: { tooltip: true } },
  {
    title: '分数',
    key: 'score',
    width: 80,
    render: (row) => row.score.toFixed(4),
  },
  { title: '类型', key: 'targetType', width: 80 },
  {
    title: '关键词',
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
</style>
