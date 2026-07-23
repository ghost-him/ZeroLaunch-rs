<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import {
  NButton,
  NCheckbox,
  NInput,
  NInputNumber,
  NSelect,
} from 'naive-ui'
import FormSection from '@/components/settings/FormSection.vue'

const OPENAI_COMPATIBLE_ID = 'openai-compatible'
const MOCK_PROVIDER_ID = 'mock'

const PROVIDER_CATALOG = [
  {
    id: OPENAI_COMPATIBLE_ID,
    label: 'OpenAI 兼容',
    hint: null as string | null,
  },
  {
    id: MOCK_PROVIDER_ID,
    label: '模拟示例',
    hint: '联调用：有其它引擎成功结果时复制其一，否则显示占位文案',
  },
] as const

const LLM_BASE_URL_PRESETS = [
  { label: 'DeepSeek', id: 'deepseek', url: 'https://api.deepseek.com' },
  { label: '智谱 GLM', id: 'glm', url: 'https://open.bigmodel.cn/api/paas/v4' },
  { label: 'OpenAI', id: 'openai', url: 'https://api.openai.com/v1' },
  { label: '硅基流动', id: 'siliconflow', url: 'https://api.siliconflow.cn/v1' },
  {
    label: '阿里云百炼',
    id: 'dashscope',
    url: 'https://dashscope.aliyuncs.com/compatible-mode/v1',
  },
  {
    label: '腾讯云 TokenHub',
    id: 'tokenhub',
    url: 'https://tokenhub.tencentmaas.com/v1',
  },
  { label: 'Kimi', id: 'kimi', url: 'https://api.moonshot.cn/v1' },
  { label: '小米 MiMo', id: 'mimo', url: 'https://api.xiaomimimo.com/v1' },
  { label: '自定义', id: 'custom', url: null },
] as const

const CUSTOM_PRESET_ID = 'custom'

const SUPPORTED_LANGUAGES = [
  'zh',
  'zh-TR',
  'yue',
  'en',
  'fr',
  'pt',
  'es',
  'ja',
  'tr',
  'ru',
  'ar',
  'ko',
  'th',
  'it',
  'de',
  'vi',
  'ms',
  'id',
] as const

const LANG_LABELS: Record<string, string> = {
  zh: '简体中文',
  'zh-TR': '繁体中文',
  yue: '粤语',
  en: '英语',
  fr: '法语',
  pt: '葡萄牙语',
  es: '西班牙语',
  ja: '日语',
  tr: '土耳其语',
  ru: '俄语',
  ar: '阿拉伯语',
  ko: '韩语',
  th: '泰语',
  it: '意大利语',
  de: '德语',
  vi: '越南语',
  ms: '马来语',
  id: '印尼语',
}

type TranslatorLocalSettings = {
  translate_mode: 'live' | 'on_enter'
  default_target: string
  enabled_providers: string[]
  request_timeout_ms: number
  llm_vendor: string
  llm_base_url: string
  llm_api_key: string
  llm_model: string
}

const props = defineProps<{
  currentSettings: unknown
}>()

const emit = defineEmits<{
  (e: 'save', settings: TranslatorLocalSettings): void
}>()

const saving = ref(false)
const dragFromIndex = ref<number | null>(null)
const dragOverIndex = ref<number | null>(null)

function labelForLang(code: string): string {
  const name = LANG_LABELS[code]
  return name ? `${name} (${code})` : code
}

function langCodeFromOption(opt: string): string {
  const t = opt.trim()
  const start = t.lastIndexOf('(')
  const end = t.lastIndexOf(')')
  if (start >= 0 && end > start + 1) {
    return t.slice(start + 1, end).trim()
  }
  return t
}

function modeFromRaw(raw: unknown): 'live' | 'on_enter' {
  if (raw === 'on_enter' || raw === '按 Enter 翻译') return 'on_enter'
  return 'live'
}

function vendorLabelFromUrl(url: string): string {
  const trimmed = url.trim()
  if (!trimmed) return '自定义'
  const match = LLM_BASE_URL_PRESETS.find((p) => p.url === trimmed)
  return match?.label ?? '自定义'
}

function vendorIdFromLabel(label: string): string {
  const match = LLM_BASE_URL_PRESETS.find((p) => p.label === label || p.id === label)
  return match?.id ?? CUSTOM_PRESET_ID
}

function providerIdFromRaw(raw: string): string {
  if (raw === 'OpenAI 兼容' || raw === OPENAI_COMPATIBLE_ID) return OPENAI_COMPATIBLE_ID
  if (raw === '模拟示例' || raw === MOCK_PROVIDER_ID) return MOCK_PROVIDER_ID
  return raw
}

function catalogIds(): string[] {
  return PROVIDER_CATALOG.map((p) => p.id)
}

/** 已启用顺序在前，其余按目录默认顺序追加 */
function buildProviderOrder(enabled: string[]): string[] {
  const catalog = catalogIds()
  const seen = new Set<string>()
  const order: string[] = []
  for (const id of enabled) {
    if (catalog.includes(id) && !seen.has(id)) {
      order.push(id)
      seen.add(id)
    }
  }
  for (const id of catalog) {
    if (!seen.has(id)) {
      order.push(id)
      seen.add(id)
    }
  }
  return order
}

function defaults(): TranslatorLocalSettings {
  return {
    translate_mode: 'live',
    default_target: 'zh',
    enabled_providers: [OPENAI_COMPATIBLE_ID],
    request_timeout_ms: 15000,
    llm_vendor: '自定义',
    llm_base_url: '',
    llm_api_key: '',
    llm_model: '',
  }
}

function fromProps(raw: unknown): TranslatorLocalSettings {
  const base = defaults()
  if (!raw || typeof raw !== 'object') return base
  const o = raw as Record<string, unknown>

  let enabledProviders: string[] = base.enabled_providers
  if (Array.isArray(o.enabled_providers)) {
    enabledProviders = (o.enabled_providers as unknown[])
      .filter((x): x is string => typeof x === 'string')
      .map(providerIdFromRaw)
    if (enabledProviders.length === 0) enabledProviders = base.enabled_providers
  }

  const vendorRaw =
    typeof o.llm_vendor === 'string' ? o.llm_vendor : vendorLabelFromUrl(String(o.llm_base_url ?? ''))

  return {
    translate_mode: modeFromRaw(o.translate_mode),
    default_target:
      typeof o.default_target === 'string'
        ? langCodeFromOption(o.default_target)
        : base.default_target,
    enabled_providers: enabledProviders,
    request_timeout_ms:
      typeof o.request_timeout_ms === 'number'
        ? o.request_timeout_ms
        : base.request_timeout_ms,
    llm_vendor: vendorRaw,
    llm_base_url: typeof o.llm_base_url === 'string' ? o.llm_base_url : base.llm_base_url,
    llm_api_key: typeof o.llm_api_key === 'string' ? o.llm_api_key : base.llm_api_key,
    llm_model: typeof o.llm_model === 'string' ? o.llm_model : base.llm_model,
  }
}

const local = reactive(fromProps(props.currentSettings))
const selectedPreset = ref(vendorIdFromLabel(local.llm_vendor))
const providerOrder = ref(buildProviderOrder(local.enabled_providers))
const enabledSet = ref(new Set(local.enabled_providers))

function syncProviderUiFromSettings(settings: TranslatorLocalSettings) {
  providerOrder.value = buildProviderOrder(settings.enabled_providers)
  enabledSet.value = new Set(settings.enabled_providers)
}

watch(
  () => props.currentSettings,
  (v) => {
    const next = fromProps(v)
    Object.assign(local, next)
    selectedPreset.value = vendorIdFromLabel(local.llm_vendor)
    syncProviderUiFromSettings(next)
  },
)

watch(selectedPreset, (id) => {
  const preset = LLM_BASE_URL_PRESETS.find((p) => p.id === id)
  if (!preset) return
  local.llm_vendor = preset.label
  if (preset.url) {
    local.llm_base_url = preset.url
  }
})

watch(
  () => local.llm_base_url,
  (url) => {
    const detected = vendorIdFromLabel(vendorLabelFromUrl(url))
    if (detected !== selectedPreset.value) {
      selectedPreset.value = detected
      local.llm_vendor = vendorLabelFromUrl(url)
    }
  },
)

const targetOptions = computed(() =>
  SUPPORTED_LANGUAGES.map((value) => ({ label: labelForLang(value), value })),
)

const translateModeOptions = [
  { label: '即时翻译', value: 'live' },
  { label: '按 Enter 翻译', value: 'on_enter' },
]

const presetOptions = LLM_BASE_URL_PRESETS.map((p) => ({
  label: p.label,
  value: p.id,
}))

const orderedProviders = computed(() =>
  providerOrder.value
    .map((id) => PROVIDER_CATALOG.find((p) => p.id === id))
    .filter((p): p is (typeof PROVIDER_CATALOG)[number] => !!p),
)

const openaiEnabled = computed(() => enabledSet.value.has(OPENAI_COMPATIBLE_ID))

watch(
  targetOptions,
  (opts) => {
    if (!opts.some((o) => o.value === local.default_target)) {
      local.default_target = opts[0]?.value ?? 'zh'
    }
  },
  { immediate: true },
)

function isProviderEnabled(id: string): boolean {
  return enabledSet.value.has(id)
}

function setProviderEnabled(id: string, checked: boolean) {
  const next = new Set(enabledSet.value)
  if (checked) {
    next.add(id)
  } else {
    if (next.size <= 1 && next.has(id)) return
    next.delete(id)
  }
  enabledSet.value = next
  local.enabled_providers = providerOrder.value.filter((pid) => next.has(pid))
}

function onDragStart(index: number, e: DragEvent) {
  dragFromIndex.value = index
  e.dataTransfer?.setData('text/plain', String(index))
  if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move'
}

function onDragOver(index: number, e: DragEvent) {
  e.preventDefault()
  if (e.dataTransfer) e.dataTransfer.dropEffect = 'move'
  dragOverIndex.value = index
}

function onDragLeave(index: number) {
  if (dragOverIndex.value === index) dragOverIndex.value = null
}

function onDrop(index: number, e: DragEvent) {
  e.preventDefault()
  const from = dragFromIndex.value
  dragFromIndex.value = null
  dragOverIndex.value = null
  if (from == null || from === index) return
  const next = [...providerOrder.value]
  const [item] = next.splice(from, 1)
  if (!item) return
  next.splice(index, 0, item)
  providerOrder.value = next
  local.enabled_providers = next.filter((id) => enabledSet.value.has(id))
}

function onDragEnd() {
  dragFromIndex.value = null
  dragOverIndex.value = null
}

async function onSave() {
  saving.value = true
  try {
    const providers = providerOrder.value.filter((id) => enabledSet.value.has(id))
    const enabled = providers.length > 0 ? providers : [OPENAI_COMPATIBLE_ID]
    local.enabled_providers = enabled
    emit('save', {
      translate_mode: local.translate_mode,
      default_target: local.default_target,
      enabled_providers: enabled,
      request_timeout_ms: local.request_timeout_ms,
      llm_vendor: local.llm_vendor,
      llm_base_url: local.llm_base_url.trim(),
      llm_api_key: local.llm_api_key,
      llm_model: local.llm_model.trim(),
    })
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <div class="translator-settings">
    <div class="form-groups">
      <FormSection title="基础" :collapsible="true">
        <div class="form-field">
          <label class="field-label">翻译触发</label>
          <div class="field-control">
            <n-select
              v-model:value="local.translate_mode"
              :options="translateModeOptions"
              class="control-full"
            />
            <p class="field-hint">即时：输入即翻译；按 Enter：确认后才请求，节省 token</p>
          </div>
        </div>
        <div class="form-field">
          <label class="field-label">默认目标语言</label>
          <div class="field-control">
            <n-select
              v-model:value="local.default_target"
              :options="targetOptions"
              filterable
              class="control-full"
            />
          </div>
        </div>
        <div class="form-field">
          <label class="field-label">超时（毫秒）</label>
          <div class="field-control">
            <n-input-number
              v-model:value="local.request_timeout_ms"
              :min="1000"
              :max="60000"
              :step="500"
              class="control-full"
            />
          </div>
        </div>
      </FormSection>

      <FormSection title="引擎" :collapsible="true">
        <div class="form-field">
          <label class="field-label">翻译引擎</label>
          <div class="field-control">
            <p class="field-hint">拖拽调整顺序，靠前的引擎优先作为主结果</p>
            <ul class="provider-list">
              <li
                v-for="(provider, index) in orderedProviders"
                :key="provider.id"
                class="provider-item"
                :class="{
                  'provider-item--over': dragOverIndex === index,
                  'provider-item--dragging': dragFromIndex === index,
                }"
                draggable="true"
                @dragstart="onDragStart(index, $event)"
                @dragover="onDragOver(index, $event)"
                @dragleave="onDragLeave(index)"
                @drop="onDrop(index, $event)"
                @dragend="onDragEnd"
              >
                <div class="provider-row">
                  <span class="drag-handle" title="拖拽调序" aria-hidden="true">⠿</span>
                  <n-checkbox
                    :checked="isProviderEnabled(provider.id)"
                    @update:checked="(v: boolean) => setProviderEnabled(provider.id, v)"
                  >
                    {{ provider.label }}
                  </n-checkbox>
                </div>
                <p v-if="provider.hint && isProviderEnabled(provider.id)" class="provider-hint">
                  {{ provider.hint }}
                </p>
                <div
                  v-if="provider.id === OPENAI_COMPATIBLE_ID && openaiEnabled"
                  class="provider-settings"
                >
                  <div class="form-field">
                    <label class="field-label">厂商预设</label>
                    <div class="field-control">
                      <n-select
                        v-model:value="selectedPreset"
                        :options="presetOptions"
                        class="control-full"
                      />
                      <p class="field-hint">
                        选择预设将立刻填入 Base URL；选「自定义」不会覆盖当前地址
                      </p>
                    </div>
                  </div>
                  <div class="form-field">
                    <label class="field-label">Base URL</label>
                    <div class="field-control">
                      <n-input
                        v-model:value="local.llm_base_url"
                        placeholder="例如 https://api.deepseek.com"
                        clearable
                        class="control-full"
                      />
                    </div>
                  </div>
                  <div class="form-field">
                    <label class="field-label">API Key</label>
                    <div class="field-control">
                      <n-input
                        v-model:value="local.llm_api_key"
                        type="password"
                        show-password-on="click"
                        placeholder="请输入 API Key"
                        clearable
                        class="control-full"
                      />
                    </div>
                  </div>
                  <div class="form-field">
                    <label class="field-label">Model</label>
                    <div class="field-control">
                      <n-input
                        v-model:value="local.llm_model"
                        placeholder="例如 deepseek-chat、moonshot-v1-8k"
                        clearable
                        class="control-full"
                      />
                    </div>
                  </div>
                </div>
              </li>
            </ul>
          </div>
        </div>
      </FormSection>
    </div>

    <div class="form-actions">
      <n-button type="primary" :loading="saving" @click="onSave">应用</n-button>
    </div>
  </div>
</template>

<style scoped>
.translator-settings {
  display: flex;
  flex-direction: column;
  min-height: 0;
  flex: 1 1 auto;
  padding: 16px 24px 0;
}

.form-groups {
  flex: 1;
  overflow-y: auto;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding-bottom: 16px;
}

.form-field {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.field-label {
  font-size: var(--font-size-sm);
  color: var(--text-primary);
}

.field-control {
  display: flex;
  flex-direction: column;
  gap: 6px;
  width: 100%;
}

.control-full {
  width: 100%;
}

.field-hint {
  margin: 0;
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
  line-height: 1.5;
}

.provider-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.provider-item {
  border: 1px solid var(--border-color);
  border-radius: 8px;
  padding: 10px 12px;
  background: var(--bg-color);
  transition: border-color 0.15s ease, opacity 0.15s ease;
}

.provider-item--over {
  border-color: var(--primary-color, #18a058);
}

.provider-item--dragging {
  opacity: 0.55;
}

.provider-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.drag-handle {
  cursor: grab;
  user-select: none;
  color: var(--text-secondary);
  font-size: 14px;
  line-height: 1;
  padding: 2px 4px;
}

.drag-handle:active {
  cursor: grabbing;
}

.provider-hint {
  margin: 6px 0 0 28px;
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
  line-height: 1.45;
}

.provider-settings {
  margin: 10px 0 0 28px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding-top: 8px;
  border-top: 1px dashed var(--border-color);
}

.form-actions {
  display: flex;
  gap: 8px;
  padding: 12px 0 16px;
  border-top: 1px solid var(--border-color);
  background-color: var(--bg-color);
  flex-shrink: 0;
}
</style>
