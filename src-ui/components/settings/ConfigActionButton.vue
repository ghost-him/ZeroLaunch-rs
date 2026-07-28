<template>
  <n-button
    v-if="actionDef && fieldAction"
    size="small"
    :loading="loading"
    :disabled="!editable"
    @click="execute"
  >
    {{ actionDef.label }}
  </n-button>
</template>

<script setup lang="ts">
import { ref, inject, onMounted } from 'vue'
import { NButton, useMessage } from 'naive-ui'
import { useI18n } from 'vue-i18n'
import { useConfigStore } from '../../stores/config-store'
import { FORM_VALUES_KEY } from '../../utils/formInjection'
import type { ConfigActionDef, EffectActionBinding, FieldAction } from '../../bridge/contract'

const props = defineProps<{
  componentId: string
  fieldAction: FieldAction
  fieldKey: string
  editable: boolean
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: unknown): void
}>()

const configStore = useConfigStore()
const message = useMessage()
const { t } = useI18n()
const loading = ref(false)
const actionDef = ref<ConfigActionDef | null>(null)
const formContext = inject(FORM_VALUES_KEY, null)

/** 判断 action 返回值是否为可读取字段的普通对象。 */
function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

/** 根据 effect action 的字段映射生成后端参数，不生成额外默认值。 */
function buildEffectParams(binding: EffectActionBinding): Record<string, unknown> {
  const values = formContext?.values.value ?? {}
  if (!binding.fieldMapping || binding.fieldMapping.length === 0) {
    return { ...values }
  }
  const params: Record<string, unknown> = {}
  for (const [sourceField, targetField] of binding.fieldMapping) {
    if (values[sourceField] !== undefined) {
      params[targetField] = values[sourceField]
    }
  }
  return params
}

/** 加载绑定 action 的声明，用于显示按钮名称和执行组件。 */
onMounted(async () => {
  const binding = props.fieldAction.binding
  try {
    const actions = await configStore.getActions(binding.component || props.componentId)
    actionDef.value = actions.find((action) => action.action === binding.action) ?? null
  } catch {
    actionDef.value = null
  }
})

/** 执行用户触发的 data/effect action，并按 action 类型处理返回值。 */
async function execute(): Promise<void> {
  const action = props.fieldAction
  const binding = action.binding
  if (!actionDef.value) return

  loading.value = true
  try {
    const params = action.kind === 'effect' ? buildEffectParams(action.binding) : undefined
    const result = await configStore.executeAction(
      binding.component || props.componentId,
      binding.action,
      params,
    )

    if (action.kind === 'data' && isRecord(result)) {
      if (action.binding.valueField in result) {
        emit('update:modelValue', result[action.binding.valueField])
      }
      if (action.binding.fieldMapping) {
        for (const [fromField, toField] of action.binding.fieldMapping) {
          const value = result[fromField]
          if (value !== undefined && formContext?.setValue) {
            formContext.setValue(toField, value)
          }
        }
      }
    } else if (action.kind === 'data' && result !== null && result !== undefined) {
      emit('update:modelValue', result)
    } else if (action.kind === 'effect' && isRecord(result) && props.fieldKey in result) {
      emit('update:modelValue', result[props.fieldKey])
    }

    if (action.kind === 'effect' && isRecord(result) && result.success === false) {
      throw new Error(String(result.message ?? t('settings.actionFailedDefault')))
    }
    message.success(t('settings.actionSuccess', { label: actionDef.value.label }))
  } catch (error) {
    message.error(t('settings.actionFailed', {
      label: actionDef.value.label,
      message: String(error),
    }))
  } finally {
    loading.value = false
  }
}
</script>
