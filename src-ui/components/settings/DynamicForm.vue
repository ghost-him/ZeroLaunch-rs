<template>
  <div class="dynamic-form">
    <div class="form-header">
      <h3>{{ schema.componentName }}</h3>
      <n-tag :bordered="false" size="small">{{ schema.componentType }}</n-tag>
    </div>
    <p class="form-desc" v-if="schema.componentDescription">{{ schema.componentDescription }}</p>

    <n-alert v-if="!schemaSupported" type="warning">
      {{ $t('settings.unsupportedSchemaVersion', { version: schema.contribution.schemaVersion }) }}
    </n-alert>
    <n-alert v-else-if="schemaError" type="warning">
      {{ $t('settings.schemaBuildFailed', { error: schemaError }) }}
    </n-alert>
    <div v-else-if="hasFields" class="form-groups">
      <FormSection
        v-for="group in groupedFields"
        :key="group.name"
        :title="group.name"
        :collapsible="group.name !== ''"
      >
        <DynamicFormField
          v-for="field in group.items"
          :key="field.key"
          :field="field"
          :component-id="schema.componentId"
          :model-value="getValue(field.key)"
          @update:model-value="(val: unknown) => setValue(field.key, val)"
        />
      </FormSection>
    </div>
    <div v-else class="empty-hint">
      <n-text depth="3">{{ $t('settings.noConfigurableFields') }}</n-text>
    </div>

    <div v-if="schemaSupported && !schemaError && hasFields" class="form-actions">
      <n-button
        v-if="commitPolicy === 'staged'"
        type="primary"
        :loading="saving"
        @click="onApply"
      >
        {{ $t('settings.save') }}
      </n-button>
      <n-text v-else depth="3">{{ $t('settings.autoSave') }}</n-text>
      <n-button :loading="resetting" @click="onReset">
        {{ $t('settings.reset') }}
      </n-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, provide, onUnmounted } from 'vue'
import { NAlert, NButton, NTag, NText, useMessage, useDialog } from 'naive-ui'
import { useI18n } from 'vue-i18n'
import DynamicFormField from './DynamicFormField.vue'
import FormSection from './FormSection.vue'
import { useConfigStore } from '../../stores/config-store'
import { FORM_VALUES_KEY } from '../../utils/formInjection'
import {
  buildFieldConfigs,
  stripTransientSettings,
  SUPPORTED_SCHEMA_VERSION,
  validateSettings,
} from '../../utils/schemaTypes'
import type { ComponentSchema } from '../../bridge/contract'
import type { FieldConfig } from '../../utils/schemaTypes'

const props = defineProps<{
  schema: ComponentSchema
  currentSettings: Record<string, unknown>
}>()

const emit = defineEmits<{
  (e: 'reload'): void
}>()

const message = useMessage()
const dialog = useDialog()
const { t } = useI18n()
const configStore = useConfigStore()
const commitPolicy = computed(() => props.schema.contribution.commitPolicy)
const schemaSupported = computed(() => props.schema.contribution.schemaVersion === SUPPORTED_SCHEMA_VERSION)
let immediateTimer: ReturnType<typeof setTimeout> | null = null
let immediateRevision = 0
let immediateSaveChain: Promise<void> = Promise.resolve()
const saving = ref(false)
const resetting = ref(false)
const localValues = ref<Record<string, unknown>>({ ...props.currentSettings })

watch(
  () => props.currentSettings,
  (newSettings) => {
    localValues.value = { ...newSettings }
  },
)

/** 从 contribution 构建字段列表，并把 schema 错误转换为可见警告。 */
const fieldConfigResult = computed<{ fields: FieldConfig[]; error: string | null }>(() => {
  try {
    return { fields: buildFieldConfigs(props.schema.contribution), error: null }
  } catch (error) {
    return {
      fields: [],
      error: error instanceof Error ? error.message : String(error),
    }
  }
})
const fields = computed(() => fieldConfigResult.value.fields)
const schemaError = computed(() => fieldConfigResult.value.error)

/** 是否存在可见的可配置字段；无字段时展示空态提示并隐藏操作按钮。 */
const hasFields = computed(() => fields.value.length > 0)

/** 记录无法构建字段配置的 schema 错误。 */
watch(schemaError, (error) => {
  if (error) console.warn(`[settings] ${props.schema.componentId}: ${error}`)
}, { immediate: true })

/** 按 group 分组 */
const groupedFields = computed(() => {
  const groups = new Map<string, FieldConfig[]>()
  for (const field of fields.value) {
    const g = field.group || ''
    if (!groups.has(g)) groups.set(g, [])
    groups.get(g)!.push(field)
  }
  for (const [, items] of groups) {
    items.sort((a, b) => a.order - b.order)
  }
  return [...groups.entries()].map(([name, items]) => ({ name, items }))
})

/** 读取字段当前值，供字段组件注入使用。 */
function getValue(key: string): unknown {
  return localValues.value[key]
}

/** 更新本地字段值，并按 commitPolicy 调度 immediate 提交。 */
function setValue(key: string, val: unknown): void {
  const nextValues = { ...localValues.value, [key]: val }
  localValues.value = nextValues
  if (commitPolicy.value === 'immediateAllowed') {
    const revision = ++immediateRevision
    if (immediateTimer) clearTimeout(immediateTimer)
    immediateTimer = setTimeout(() => {
      immediateTimer = null
      immediateSaveChain = immediateSaveChain
        .catch(() => {})
        .then(async () => {
          if (revision === immediateRevision) await applySettings(nextValues)
        })
    }, 150)
  }
}

provide(FORM_VALUES_KEY, { getValue, setValue, values: localValues })

/** 校验并提交当前配置值，供 staged 和 immediateAllowed 共用。 */
async function applySettings(settings: Record<string, unknown>): Promise<void> {
  const persistedSettings = stripTransientSettings(props.schema.contribution, settings)
  const validationError = validateSettings(props.schema.contribution, persistedSettings)
  if (validationError) {
    message.error(t('settings.schemaValidationFailed', { path: validationError }))
    return
  }
  saving.value = true
  try {
    await configStore.applySettings(props.schema.componentId, persistedSettings)
    message.success(t('settings.saveSuccess'))
  } catch {
    message.error(t('settings.saveFailed'))
  } finally {
    saving.value = false
  }
}

/** 提交 staged 模式下的当前设置。 */
async function onApply(): Promise<void> {
  await applySettings(localValues.value)
}

/** 重置组件设置，并取消尚未提交的 immediate 更新。 */
async function onReset() {
  dialog.warning({
    title: t('settings.resetConfirmTitle'),
    content: t('settings.resetConfirmContent'),
    positiveText: t('settings.resetConfirmPositive'),
    negativeText: t('settings.resetConfirmNegative'),
    onPositiveClick: async () => {
      immediateRevision += 1
      if (immediateTimer) {
        clearTimeout(immediateTimer)
        immediateTimer = null
      }
      await immediateSaveChain.catch(() => {})
      resetting.value = true
      try {
        await configStore.resetSettings(props.schema.componentId)
        const settings = await configStore.getSettings(props.schema.componentId)
        localValues.value = { ...(settings as Record<string, unknown>) }
        message.success(t('settings.resetSuccess'))
        emit('reload')
      } catch {
        message.error(t('settings.resetFailed'))
      } finally {
        resetting.value = false
      }
    },
  })
}

/** 组件卸载时清理待提交的 immediate 状态。 */
onUnmounted(() => {
  immediateRevision += 1
  if (immediateTimer) {
    clearTimeout(immediateTimer)
    immediateTimer = null
  }
})
</script>

<style scoped>
.dynamic-form {
  display: flex;
  flex-direction: column;
  min-height: 0;
  flex: 1 1 auto;
  padding: 16px 24px 0;
}
.form-header {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 4px;
  flex-shrink: 0;
}
.form-header h3 {
  font-size: var(--font-size-lg);
  margin: 0;
}
.form-desc {
  color: var(--text-secondary);
  font-size: var(--font-size-sm);
  margin-bottom: 16px;
  flex-shrink: 0;
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
.form-actions {
  display: flex;
  gap: 8px;
  padding: 12px 0 16px;
  border-top: 1px solid var(--border-color);
  background-color: var(--bg-color);
  flex-shrink: 0;
}
.empty-hint {
  flex: 1;
  min-height: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 32px 0;
}
</style>
