<template>
  <div class="array-table-wrap">
    <table class="array-table">
      <thead>
        <tr>
          <th v-for="fd in subFields" :key="fd.key">{{ resolveText(fd.label) }}</th>
          <th v-if="subFields.length > 0 && !field.readOnly" class="col-action">{{ $t('common.actions') }}</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="(_item, idx) in listValue" :key="idx">
          <td v-for="fd in subFields" :key="fd.key">
            <DynamicFormField
              :field="fieldDefToConfig(fd, field.readOnly)"
              :component-id="componentId"
              :show-label="false"
              :model-value="getField(idx, fd.key)"
              @update:model-value="(val: unknown) => setField(idx, fd.key, val)"
            />
          </td>
          <td v-if="subFields.length > 0 && !field.readOnly" class="col-action">
            <n-button
              text type="error" size="tiny"
              :disabled="!canRemoveArrayItem(field.schema, listValue.length)"
              @click="onRemove(idx)"
            >
              {{ $t('common.delete') }}
            </n-button>
          </td>
        </tr>
      </tbody>
    </table>
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
import { resolveText } from '../../../../i18n'
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

const listValue = computed<unknown[]>(() => {
  if (Array.isArray(props.modelValue)) return props.modelValue as unknown[]
  return []
})

/** 读取表格字段值；缺失值保持 undefined，由后端决定最终语义。 */
function getField(idx: number, key: string): unknown {
  const item = listValue.value[idx]
  if (item && typeof item === 'object' && !Array.isArray(item)) {
    return (item as Record<string, unknown>)[key]
  }
  return undefined
}

/** 更新表格字段并保持父级只读约束。 */
function setField(idx: number, key: string, val: unknown): void {
  if (props.field.readOnly) return
  const arr = [...listValue.value]
  const item = arr[idx]
  if (item && typeof item === 'object' && !Array.isArray(item)) {
    arr[idx] = { ...(item as Record<string, unknown>), [key]: val }
  }
  emit('update:modelValue', arr)
}

/** 添加一个符合 item schema 默认值的条目。 */
function onAdd(): void {
  if (!canAddArrayItem(props.field.schema, listValue.value.length) || props.field.readOnly || !itemSchema.value) return
  const arr = [...listValue.value]
  arr.push(getDefaultArrayItem(itemSchema.value))
  emit('update:modelValue', arr)
}

/** 删除条目并遵守 minItems 约束。 */
function onRemove(idx: number): void {
  if (!canRemoveArrayItem(props.field.schema, listValue.value.length) || props.field.readOnly) return
  const arr = [...listValue.value]
  arr.splice(idx, 1)
  emit('update:modelValue', arr)
}
</script>

<style scoped>
.array-table-wrap {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.array-table {
  width: 100%;
  border-collapse: collapse;
}
.array-table th,
.array-table td {
  border: 1px solid var(--border-color);
  padding: 4px 8px;
  font-size: var(--font-size-sm);
}
.array-table th {
  background-color: var(--table-header-bg);
  font-weight: 600;
}
.col-action {
  width: 50px;
  text-align: center;
}
.table-path-row {
  display: flex;
  gap: 4px;
}
</style>
