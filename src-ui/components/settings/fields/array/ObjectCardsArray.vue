<template>
  <div class="object-cards">
    <div v-for="(_item, idx) in listValue" :key="idx" class="object-card">
      <div class="object-card-header">
        <span class="card-index">#{{ idx + 1 }}</span>
        <n-button
          text type="error" size="tiny"
          :disabled="field.readOnly || !canRemoveArrayItem(field.schema, listValue.length)"
          @click="onRemove(idx)"
        >
          {{ $t('common.delete') }}
        </n-button>
      </div>
      <div v-for="fd in subFields" :key="fd.key" class="object-card-field">
        <DynamicFormField
          :field="fdToConfig(fd, field.readOnly)"
          :component-id="componentId"
          :model-value="getField(idx, fd.key)"
          @update:model-value="(val: unknown) => setField(idx, fd.key, val)"
        />
      </div>
    </div>
    <n-button
      size="small"
      :disabled="field.readOnly || !canAddArrayItem(field.schema, listValue.length)"
      @click="onAdd"
    >
      {{ $t('common.add') }}
    </n-button>
  </div>
</template>
<script setup lang="ts">
import { computed } from 'vue'
import { NButton } from 'naive-ui'
import DynamicFormField from '../../DynamicFormField.vue'
import {
  canAddArrayItem,
  canRemoveArrayItem,
  getArrayItemSchema,
  getDefaultArrayItem,
  getObjectFieldDefs,
  fieldDefToConfig,
} from '../../../../utils/schemaTypes'
import type { FieldConfig } from '../../../../utils/schemaTypes'

const props = defineProps<{
  field: FieldConfig
  componentId: string
  modelValue: unknown
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: unknown): void
}>()

const itemSchema = computed(() => getArrayItemSchema(props.field.schema))

const subFields = computed(() => itemSchema.value ? getObjectFieldDefs(itemSchema.value) : [])
const fdToConfig = fieldDefToConfig

const listValue = computed<unknown[]>(() => {
  if (Array.isArray(props.modelValue)) return props.modelValue as unknown[]
  return []
})

/** 读取对象数组中的字段值；缺失值保持 undefined，由后端决定最终语义。 */
function getField(idx: number, key: string): unknown {
  const item = listValue.value[idx]
  if (item && typeof item === 'object' && !Array.isArray(item)) {
    return (item as Record<string, unknown>)[key]
  }
  return undefined
}

/** 更新对象数组中的字段并保持数组引用不可变。 */
function setField(idx: number, key: string, val: unknown): void {
  if (props.field.readOnly) return
  const arr = [...listValue.value]
  const item = arr[idx]
  if (item && typeof item === 'object' && !Array.isArray(item)) {
    arr[idx] = { ...(item as Record<string, unknown>), [key]: val }
  }
  emit('update:modelValue', arr)
}

/** 添加一个符合 item schema 默认值的对象。 */
function onAdd(): void {
  if (!canAddArrayItem(props.field.schema, listValue.value.length) || props.field.readOnly || !itemSchema.value) return
  const arr = [...listValue.value]
  arr.push(getDefaultArrayItem(itemSchema.value))
  emit('update:modelValue', arr)
}

/** 删除对象数组中的指定项，并遵守 minItems 约束。 */
function onRemove(idx: number): void {
  if (!canRemoveArrayItem(props.field.schema, listValue.value.length) || props.field.readOnly) return
  const arr = [...listValue.value]
  arr.splice(idx, 1)
  emit('update:modelValue', arr)
}
</script>

<style scoped>
.object-cards {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.object-card {
  border: 1px solid var(--border-color);
  border-radius: 6px;
  padding: 8px 12px;
}
.object-card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}
.card-index {
  font-weight: 600;
  font-size: var(--font-size-sm);
}
.object-card-field {
  margin-bottom: 4px;
}
</style>
