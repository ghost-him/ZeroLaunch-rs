<script setup lang="ts">
import { computed } from 'vue'
import { NButton, NCollapse, NCollapseItem, NTag } from 'naive-ui'
import type { ResultAction } from '@/bridge/contract'
import { useSearchStore } from '@/stores/search-store'

interface SenseEntry {
  label?: string | null
  text: string
}

interface TranslationEntry {
  providerId: string
  providerName: string
  text: string
  phonetic?: string | null
  computerSense?: string | null
  moreSenses?: SenseEntry[]
  detectedSource?: string | null
  error?: string | null
}

interface TranslatorPanelData {
  query?: {
    text?: string
    source?: string
    target?: string
    raw?: string
  }
  primary?: TranslationEntry | null
  alternatives?: TranslationEntry[]
  status?: 'empty' | 'ready' | 'ok' | 'partial' | 'error'
  message?: string | null
}

const props = defineProps<{
  data: TranslatorPanelData
  actions: ResultAction[]
}>()

const searchStore = useSearchStore()

const status = computed(() => props.data?.status ?? 'empty')
const message = computed(() => props.data?.message ?? null)
const queryText = computed(() => props.data?.query?.text ?? '')
const querySource = computed(() => props.data?.query?.source ?? '?')
const queryTarget = computed(() => props.data?.query?.target ?? '?')
const primary = computed(() => props.data?.primary ?? null)
const primaryText = computed(() => primary.value?.text ?? '')
const primaryError = computed(() => primary.value?.error ?? null)
const primaryPhonetic = computed(() => primary.value?.phonetic?.trim() ?? '')
const primaryComputerSense = computed(() => primary.value?.computerSense?.trim() ?? '')
const primaryMoreSenses = computed(() => (primary.value?.moreSenses ?? []).slice(0, 4))
const alternatives = computed(() => props.data?.alternatives ?? [])

async function copyToClipboard(text: string) {
  try {
    await navigator.clipboard.writeText(text)
  } catch (error) {
    console.warn('[翻译面板] 剪贴板写入失败:', error)
  }
}

async function executeAction(action: ResultAction) {
  if (action.id === 'copy_primary' && primaryText.value) {
    await copyToClipboard(primaryText.value)
  }
  await searchStore.doConfirm(0, action.id)
}
</script>

<template>
  <div class="translator-panel">
    <!-- 空引导态 -->
    <div v-if="status === 'empty'" class="tr-empty">
      <div class="tr-empty-title">翻译</div>
      <div class="tr-empty-hint">用法示例：</div>
      <ul class="tr-usage-list">
        <li><code>fy hello</code> — 自动检测源语</li>
        <li><code>fy en 你好</code> — 指定目标语</li>
        <li><code>fy zh en hello</code> — 指定源语与目标语</li>
      </ul>
    </div>

    <!-- 待确认翻译（按 Enter 模式） -->
    <div v-else-if="status === 'ready'" class="tr-display tr-ready">
      <div class="tr-meta">
        <span>{{ querySource }} → {{ queryTarget }}</span>
        <span v-if="queryText" class="tr-meta-text">{{ queryText }}</span>
      </div>
      <div class="tr-primary tr-ready-hint">
        {{ message || '按 Enter 翻译' }}
      </div>
    </div>

    <!-- 错误态 -->
    <div v-else-if="status === 'error' && !primaryText" class="tr-display tr-error">
      {{ message || '翻译失败，请稍后重试' }}
    </div>

    <!-- 正常 / 部分成功 -->
    <template v-else>
      <div class="tr-display">
        <div class="tr-meta">
          <span>{{ querySource }} → {{ queryTarget }}</span>
          <span v-if="queryText" class="tr-meta-text">{{ queryText }}</span>
        </div>

        <div class="tr-primary" :class="{ error: !!primaryError }">
          {{ primaryError || primaryText || message || '' }}
        </div>

        <div v-if="primaryPhonetic" class="tr-phonetic">
          {{ primaryPhonetic }}
        </div>

        <div v-if="primaryComputerSense" class="tr-computer">
          <n-tag size="small" :bordered="false">计算机</n-tag>
          <span class="tr-computer-text">{{ primaryComputerSense }}</span>
        </div>

        <div v-if="primaryMoreSenses.length > 0" class="tr-more-senses">
          <div
            v-for="(sense, index) in primaryMoreSenses"
            :key="index"
            class="tr-sense-row"
          >
            <span v-if="sense.label" class="tr-sense-label">{{ sense.label }}</span>
            <span class="tr-sense-text">{{ sense.text }}</span>
          </div>
        </div>
      </div>

      <div v-if="alternatives.length > 0" class="tr-alts">
        <n-collapse>
          <n-collapse-item title="其他引擎" name="alts">
            <div
              v-for="alt in alternatives"
              :key="alt.providerId"
              class="tr-alt-row"
            >
              <span class="tr-alt-name">{{ alt.providerName }}</span>
              <span v-if="alt.error" class="tr-alt-error">{{ alt.error }}</span>
              <span v-else class="tr-alt-text">{{ alt.text }}</span>
            </div>
          </n-collapse-item>
        </n-collapse>
      </div>
    </template>

    <div v-if="actions.length > 0" class="tr-actions">
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

<style scoped>
.translator-panel {
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.tr-empty-title {
  font-size: var(--font-size-base);
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: 8px;
}

.tr-empty-hint {
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
  margin-bottom: 4px;
}

.tr-usage-list {
  margin: 0;
  padding-left: 18px;
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.tr-usage-list code {
  font-family: var(--font-mono, ui-monospace, monospace);
  color: var(--text-primary);
}

.tr-display {
  background: var(--bg-secondary);
  border-radius: var(--radius-sm);
  padding: 16px;
  min-height: 60px;
  display: flex;
  flex-direction: column;
  justify-content: center;
  gap: 8px;
}

.tr-ready-hint {
  color: var(--text-secondary);
  font-size: var(--font-size-sm);
}

.tr-display.tr-error {
  color: #d03050;
  font-size: var(--font-size-base);
  font-weight: 400;
}

.tr-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 4px 8px;
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
  word-break: break-word;
}

.tr-meta-text {
  word-break: break-word;
}

.tr-primary {
  font-size: 28px;
  font-weight: 600;
  color: var(--text-primary);
  word-break: break-word;
}

.tr-primary.error {
  font-size: var(--font-size-base);
  color: #d03050;
  font-weight: 400;
}

.tr-phonetic {
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
}

.tr-computer {
  display: flex;
  align-items: flex-start;
  gap: 8px;
}

.tr-computer-text {
  font-size: var(--font-size-base);
  color: var(--text-primary);
  word-break: break-word;
}

.tr-more-senses {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.tr-sense-row {
  display: flex;
  flex-wrap: wrap;
  gap: 4px 8px;
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
}

.tr-sense-label {
  flex-shrink: 0;
}

.tr-sense-text {
  word-break: break-word;
}

.tr-alt-row {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 8px 0;
}

.tr-alt-row + .tr-alt-row {
  border-top: 1px solid var(--border-color, rgba(0, 0, 0, 0.06));
}

.tr-alt-name {
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
}

.tr-alt-text {
  font-size: var(--font-size-base);
  color: var(--text-primary);
  word-break: break-word;
}

.tr-alt-error {
  font-size: var(--font-size-sm);
  color: #d03050;
}

.tr-actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
}
</style>
