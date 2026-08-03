<script setup lang="ts">
import { computed, reactive, ref, watch, onMounted } from 'vue'
import { configGetSchema } from '@/bridge/commands'
import {
  NButton,
  NCheckbox,
  NCollapse,
  NCollapseItem,
  NInput,
  NInputNumber,
  NSelect,
} from 'naive-ui'
import { useI18n } from 'vue-i18n'
import FormSection from '@/components/settings/FormSection.vue'

const { t } = useI18n()

const OPENAI_COMPATIBLE_ID = 'openai-compatible'
const MOCK_PROVIDER_ID = 'mock'

// 引擎目录：labelKey 仅用于展示；引擎 ID（id）为持久化值。
const PROVIDER_CATALOG = [
  {
    id: OPENAI_COMPATIBLE_ID,
    labelKey: 'translator.providerOpenaiCompatible',
    hintKey: null as string | null,
  },
  {
    id: MOCK_PROVIDER_ID,
    labelKey: 'translator.providerMock',
    hintKey: 'translator.providerMockHint',
  },
] as const

// 厂商预设展示名：label 为持久化值（与后端默认预设一致，禁止翻译），labelKey 仅用于展示；
// 用户新增的厂商无对应 key 时直接显示原始 label。
// 预设列表（label → Base URL）为持久化设置 llm_vendor_options（用户可增删改），
// 由后端下发并在应用时经 normalize 校验；前端不再维护 URL 镜像。
const VENDOR_LABEL_KEYS: Record<string, string> = {
  DeepSeek: 'translator.vendorDeepSeek',
  '智谱 GLM': 'translator.vendorZhipu',
  OpenAI: 'translator.vendorOpenAI',
  '硅基流动': 'translator.vendorSiliconFlow',
  '阿里云百炼': 'translator.vendorBailian',
  '腾讯云 TokenHub': 'translator.vendorTokenHub',
  Kimi: 'translator.vendorKimi',
  '小米 MiMo': 'translator.vendorMiMo',
  自定义: 'translator.vendorCustom',
}

/** 厂商预设（label → Base URL）；url 为空串表示无预设地址（schema 校验不接受 null）。 */
type VendorOption = { label: string; url: string }

type TranslatorLocalSettings = {
  translate_mode: 'live' | 'on_enter'
  default_target: string
  enabled_providers: string[]
  request_timeout_ms: number
  live_debounce_secs: number
  llm_vendor: string
  llm_vendor_options: VendorOption[]
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

function modeFromRaw(raw: unknown): 'live' | 'on_enter' {
  // get_settings 当前直接返回代码值（如 "live"），不包含标签
  if (raw === 'on_enter') return 'on_enter'
  return 'live'
}

/** 从设置 DTO 读取厂商预设（持久化字段，用户可编辑）；非法条目剔除，URL 缺省为空串。 */
function parseVendorOptions(raw: unknown): VendorOption[] {
  const arr = (raw as Record<string, unknown> | null)?.['llm_vendor_options']
  if (!Array.isArray(arr)) return []
  return arr
    .filter(
      (x): x is { label: unknown; url?: unknown } =>
        typeof x === 'object' &&
        x !== null &&
        typeof (x as { label: unknown }).label === 'string',
    )
    .map((x) => ({
      label: (x.label as string).trim(),
      url: typeof x.url === 'string' ? x.url.trim() : '',
    }))
    .filter((p) => p.label.length > 0)
}

function vendorLabelFromUrl(url: string, options: VendorOption[]): string {
  // 返回持久化枚举值（厂商 label），非展示文本。
  const trimmed = url.trim()
  if (!trimmed) return '自定义'
  const match = options.find((p) => p.url === trimmed)
  return match?.label ?? '自定义'
}

function providerIdFromRaw(raw: string): string {
  // get_settings 当前直接返回引擎 ID（如 "openai-compatible"），不包含标签
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
    live_debounce_secs: 0.5,
    llm_vendor: '自定义',
    llm_vendor_options: [],
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

  const vendorOptions = parseVendorOptions(o)
  const vendorRaw =
    typeof o.llm_vendor === 'string'
      ? o.llm_vendor
      : vendorLabelFromUrl(String(o.llm_base_url ?? ''), vendorOptions)

  return {
    translate_mode: modeFromRaw(o.translate_mode),
    default_target:
      typeof o.default_target === 'string'
        ? o.default_target.trim()
        : base.default_target,
    enabled_providers: enabledProviders,
    request_timeout_ms:
      typeof o.request_timeout_ms === 'number'
        ? o.request_timeout_ms
        : base.request_timeout_ms,
    live_debounce_secs:
      typeof o.live_debounce_secs === 'number'
        ? o.live_debounce_secs
        : base.live_debounce_secs,
    llm_vendor: vendorRaw,
    llm_vendor_options: vendorOptions,
    llm_base_url: typeof o.llm_base_url === 'string' ? o.llm_base_url : base.llm_base_url,
    llm_api_key: typeof o.llm_api_key === 'string' ? o.llm_api_key : base.llm_api_key,
    llm_model: typeof o.llm_model === 'string' ? o.llm_model : base.llm_model,
  }
}

const local = reactive(fromProps(props.currentSettings))
const selectedPreset = ref(local.llm_vendor)
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
    selectedPreset.value = local.llm_vendor
    syncProviderUiFromSettings(next)
  },
)

watch(selectedPreset, (label) => {
  const preset = local.llm_vendor_options.find((p) => p.label === label)
  if (!preset) return
  local.llm_vendor = preset.label
  if (preset.url) {
    local.llm_base_url = preset.url
  }
})

watch(
  () => local.llm_base_url,
  (url) => {
    const detected = vendorLabelFromUrl(url, local.llm_vendor_options)
    if (detected !== selectedPreset.value) {
      selectedPreset.value = detected
      local.llm_vendor = detected
    }
  },
)

/** 由后端 schema default_target 的 enumLabels 填充。 */
const languageOptions = ref<{ label: string; value: string }[]>([])

onMounted(async () => {
  try {
    const schema = await configGetSchema('translator')
    const field = schema.contribution.properties?.['default_target']
    if (field?.type === 'string' && field.enum) {
      languageOptions.value = field.enum.map((v, i) => ({
        label: field.enumLabels?.[i] ?? v,
        value: v,
      }))
    }
  } catch {
    // schema 加载失败时语言选项为空列表，由 watch 回退到默认值
  }
})

watch(
  languageOptions,
  (opts) => {
    if (opts.length > 0 && !opts.some((o) => o.value === local.default_target)) {
      local.default_target = opts[0].value
    }
  },
  { immediate: true },
)

// 选项数组用 computed：随界面语言切换（t 响应 locale）重新求值。
const translateModeOptions = computed(() => [
  { label: t('translator.modeLive'), value: 'live' },
  { label: t('translator.modeOnEnter'), value: 'on_enter' },
])

const presetOptions = computed(() => [
  ...local.llm_vendor_options.map((p) => ({
    label: t(VENDOR_LABEL_KEYS[p.label] ?? p.label),
    value: p.label,
  })),
  { label: t('translator.vendorCustom'), value: '自定义' },
])

const orderedProviders = computed(() =>
  providerOrder.value
    .map((id) => PROVIDER_CATALOG.find((p) => p.id === id))
    .filter((p): p is (typeof PROVIDER_CATALOG)[number] => !!p),
)

const openaiEnabled = computed(() => enabledSet.value.has(OPENAI_COMPATIBLE_ID))

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

/** 新增一条空白厂商预设（label 留待用户填写）。 */
function addVendorPreset() {
  local.llm_vendor_options = [...local.llm_vendor_options, { label: '', url: '' }]
}

/** 删除指定厂商预设；若删除的是当前选中厂商，立即回落「自定义」。 */
function removeVendorPreset(index: number) {
  const removed = local.llm_vendor_options[index]
  local.llm_vendor_options = local.llm_vendor_options.filter((_, i) => i !== index)
  if (removed && local.llm_vendor === removed.label) {
    local.llm_vendor = '自定义'
    selectedPreset.value = '自定义'
  }
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
      live_debounce_secs: local.live_debounce_secs,
      llm_vendor: local.llm_vendor,
      // 空 label 条目由后端 normalize 剔除；空 URL 以空串持久化（schema 不接受 null）。
      llm_vendor_options: local.llm_vendor_options
        .map((p) => ({ label: p.label.trim(), url: p.url.trim() }))
        .filter((p) => p.label.length > 0),
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
      <FormSection :title="$t('translator.sectionBasic')" :collapsible="true">
        <div class="form-field">
          <label class="field-label">{{ $t('translator.translateTrigger') }}</label>
          <div class="field-control">
            <n-select
              v-model:value="local.translate_mode"
              :options="translateModeOptions"
              class="control-full"
            />
            <p class="field-hint">{{ $t('translator.triggerHint') }}</p>
          </div>
        </div>
        <div class="form-field">
          <label class="field-label">{{ $t('translator.defaultTargetLanguage') }}</label>
          <div class="field-control">
            <n-select
              v-model:value="local.default_target"
              :options="languageOptions"
              filterable
              class="control-full"
            />
          </div>
        </div>
        <div class="form-field">
          <label class="field-label">{{ $t('translator.timeoutMs') }}</label>
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
        <div class="form-field">
          <label class="field-label">{{ $t('translator.liveDebounceSecs') }}</label>
          <div class="field-control">
            <n-input-number
              v-model:value="local.live_debounce_secs"
              :min="0.1"
              :max="5.0"
              :step="0.1"
              class="control-full"
            />
            <p class="field-hint">{{ $t('translator.liveDebounceHint') }}</p>
          </div>
        </div>
      </FormSection>

      <FormSection :title="$t('translator.sectionEngine')" :collapsible="true">
        <div class="form-field">
          <label class="field-label">{{ $t('translator.translateEngine') }}</label>
          <div class="field-control">
            <p class="field-hint">{{ $t('translator.engineOrderHint') }}</p>
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
                  <span class="drag-handle" :title="$t('translator.dragToReorder')" aria-hidden="true">⠿</span>
                  <n-checkbox
                    :checked="isProviderEnabled(provider.id)"
                    @update:checked="(v: boolean) => setProviderEnabled(provider.id, v)"
                  >
                    {{ $t(provider.labelKey) }}
                  </n-checkbox>
                </div>
                <p v-if="provider.hintKey && isProviderEnabled(provider.id)" class="provider-hint">
                  {{ $t(provider.hintKey) }}
                </p>
                <div
                  v-if="provider.id === OPENAI_COMPATIBLE_ID && openaiEnabled"
                  class="provider-settings"
                >
                  <div class="form-field">
                    <label class="field-label">{{ $t('translator.vendorPreset') }}</label>
                    <div class="field-control">
                      <n-select
                        v-model:value="selectedPreset"
                        :options="presetOptions"
                        class="control-full"
                      />
                      <p class="field-hint">
                        {{ $t('translator.vendorPresetHint') }}
                      </p>
                    </div>
                  </div>
                  <div class="form-field">
                    <div class="field-control">
                      <!-- 预设列表可折叠，默认折叠：列表较长，展开仅在需要编辑时 -->
                      <n-collapse>
                        <n-collapse-item
                          :title="$t('translator.vendorPresets')"
                          name="vendor-presets"
                        >
                          <div
                            v-for="(preset, index) in local.llm_vendor_options"
                            :key="index"
                            class="preset-row"
                          >
                            <n-input
                              v-model:value="preset.label"
                              :placeholder="$t('translator.vendorPresetLabelPlaceholder')"
                              class="preset-label-input"
                            />
                            <n-input
                              v-model:value="preset.url"
                              :placeholder="$t('translator.vendorPresetUrlPlaceholder')"
                              class="preset-url-input"
                            />
                            <n-button
                              size="small"
                              quaternary
                              type="error"
                              @click="removeVendorPreset(index)"
                            >
                              {{ $t('translator.removeVendorPreset') }}
                            </n-button>
                          </div>
                          <n-button size="small" @click="addVendorPreset">
                            {{ $t('translator.addVendorPreset') }}
                          </n-button>
                        </n-collapse-item>
                      </n-collapse>
                    </div>
                  </div>
                  <div class="form-field">
                    <label class="field-label">{{ $t('translator.baseUrl') }}</label>
                    <div class="field-control">
                      <n-input
                        v-model:value="local.llm_base_url"
                        :placeholder="$t('translator.baseUrlPlaceholder')"
                        clearable
                        class="control-full"
                      />
                    </div>
                  </div>
                  <div class="form-field">
                    <label class="field-label">{{ $t('translator.apiKey') }}</label>
                    <div class="field-control">
                      <n-input
                        v-model:value="local.llm_api_key"
                        type="password"
                        show-password-on="click"
                        :placeholder="$t('translator.apiKeyPlaceholder')"
                        clearable
                        class="control-full"
                      />
                    </div>
                  </div>
                  <div class="form-field">
                    <label class="field-label">{{ $t('translator.model') }}</label>
                    <div class="field-control">
                      <n-input
                        v-model:value="local.llm_model"
                        :placeholder="$t('translator.modelPlaceholder')"
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
      <n-button type="primary" :loading="saving" @click="onSave">{{ $t('translator.apply') }}</n-button>
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

.preset-row {
  display: flex;
  align-items: center;
  gap: 6px;
}

.preset-label-input {
  width: 160px;
  flex-shrink: 0;
}

.preset-url-input {
  flex: 1;
  min-width: 0;
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
