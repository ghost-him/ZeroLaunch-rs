<template>
  <div class="object-field">
    <DynamicFormField
      v-for="fd in subFields"
      :key="fd.key"
      :field="fieldDefToConfig(fd, field.readOnly)"
      :component-id="componentId"
      :model-value="getValue(fd.key)"
      @update:model-value="(value: unknown) => setValue(fd.key, value)"
    />
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import DynamicFormField from '../DynamicFormField.vue'
import {
  fieldDefToConfig,
  getObjectFieldDefs,
} from '../../../utils/schemaTypes'
import type { FieldConfig } from '../../../utils/schemaTypes'

const props = defineProps<{
  field: FieldConfig
  componentId: string
  modelValue: unknown
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: unknown): void
}>()

const objectValue = computed<Record<string, unknown>>(() => {
  if (
    props.modelValue !== null &&
    typeof props.modelValue === 'object' &&
    !Array.isArray(props.modelValue)
  ) {
    return props.modelValue as Record<string, unknown>
  }
  return {}
})

const subFields = computed(() => getObjectFieldDefs(props.field.schema))

/** 读取对象字段值；缺失值保持 undefined，由后端校验和默认配置决定。 */
function getValue(key: string): unknown {
  return objectValue.value[key]
}

/** 更新对象字段并保持父级 readOnly 不可写。 */
function setValue(key: string, value: unknown): void {
  if (props.field.readOnly) return
  emit('update:modelValue', { ...objectValue.value, [key]: value })
}
</script>

<style scoped>
.object-field {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
</style>
