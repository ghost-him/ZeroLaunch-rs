<template>
  <div class="form-field" v-if="field.visible">
    <label class="field-label" v-if="showLabel">
      {{ resolveText(field.label) }}
      <span class="field-desc" v-if="field.description">
        — {{ resolveText(field.description) }}
      </span>
    </label>
    <div class="field-control">
      <n-alert v-if="unsupportedReason" type="warning">
        {{ $t('settings.unsupportedSchema', { field: resolveText(field.label), reason: unsupportedReason }) }}
      </n-alert>
      <component
        v-else-if="fieldComponent"
        :is="fieldComponent"
        :field="field"
        :component-id="componentId"
        :model-value="modelValue"
        @update:model-value="emit('update:modelValue', $event)"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, watch } from 'vue'
import type { Component } from 'vue'
import { useI18n } from 'vue-i18n'
import { resolveText } from '../../i18n'
import { NAlert } from 'naive-ui'
import { getFieldRenderInfo } from '../../utils/schemaTypes'
import type { FieldConfig } from '../../utils/schemaTypes'
import TextField from './fields/TextField.vue'
import NumberField from './fields/NumberField.vue'
import BooleanField from './fields/BooleanField.vue'
import SelectField from './fields/SelectField.vue'
import ColorField from './fields/ColorField.vue'
import PathField from './fields/PathField.vue'
import ArrayField from './fields/ArrayField.vue'
import ImageField from './fields/ImageField.vue'
import ObjectField from './fields/ObjectField.vue'

const props = withDefaults(defineProps<{
  field: FieldConfig
  componentId: string
  modelValue: unknown
  showLabel?: boolean
}>(), {
  showLabel: true,
})

const emit = defineEmits<{
  (e: 'update:modelValue', value: unknown): void
}>()
const { t } = useI18n()

/** 通过 schema 工具集中解析字段渲染器和不支持原因。 */
const renderInfo = computed(() => getFieldRenderInfo(props.field))

/** 将 schema/widget 组合错误转换为本地化提示。 */
const unsupportedReason = computed<string | null>(() => {
  const info = renderInfo.value
  if (!info.error) return null
  if (info.error === 'unknownSchema') {
    return t('settings.unknownSchemaType', { type: info.schemaType })
  }
  return t('settings.unsupportedWidget', {
    schemaType: info.schemaType,
    widget: info.widgetKind ?? 'unknown',
  })
})

/** 将集中分派结果映射为 Vue 字段组件。 */
const fieldComponent = computed<Component | null>(() => {
  switch (renderInfo.value.kind) {
    case 'text': return TextField
    case 'number': return NumberField
    case 'boolean': return BooleanField
    case 'select': return SelectField
    case 'color': return ColorField
    case 'path': return PathField
    case 'image': return ImageField
    case 'array': return ArrayField
    case 'object': return ObjectField
    default: return null
  }
})

/** 将不支持的 schema 路由记录到控制台，避免错误被静默吞掉。 */
watch(unsupportedReason, (reason) => {
  if (reason) {
    console.warn(`[settings] ${props.field.key}: ${reason}`)
  }
}, { immediate: true })
</script>

<style scoped>
.form-field {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.field-label {
  font-size: var(--font-size-sm);
  color: var(--text-primary);
}
.field-desc {
  color: var(--text-secondary);
  font-weight: 400;
}
.field-control {
  display: flex;
  flex-direction: column;
}
</style>
