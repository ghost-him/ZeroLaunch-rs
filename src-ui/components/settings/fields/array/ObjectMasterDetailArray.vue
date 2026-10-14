<template>
  <div class="array-master-detail">
    <div class="md-list">
      <div
        v-for="(item, idx) in listValue"
        :key="rowKey(item, idx)"
        class="md-list-item"
        :class="{ active: selectedIndex === idx }"
        @click="selectedIndex = idx"
      >
        <span>{{ summary(idx) }}</span>
        <n-button
          text type="error" size="tiny"
          :disabled="field.readOnly || !canRemoveArrayItem(field.schema, listValue.length)"
          @click.stop="onRemove(idx)"
        >
          {{ $t('common.delete') }}
        </n-button>
      </div>
      <n-button
        size="small"
        :disabled="field.readOnly || !canAddArrayItem(field.schema, listValue.length)"
        @click="onAdd"
      >
        {{ $t('common.add') }}
      </n-button>
    </div>
    <div class="md-detail">
      <template v-if="selectedIndex < listValue.length">
        <div v-for="fd in visibleSubFields" :key="fd.key" class="md-field">
          <DynamicFormField
            :field="fdToConfig(fd, field.readOnly)"
            :component-id="componentId"
            :model-value="getField(selectedIndex, fd.key)"
            @update:model-value="(val: unknown) => setField(selectedIndex, fd.key, val)"
          />
        </div>
        <DetailPreviewPanel
          v-if="field.detailAction && selectedParamValue"
          :component-id="componentId"
          :detail-action="field.detailAction"
          :param-value="selectedParamValue"
          :read-only="field.readOnly"
        />
      </template>
      <n-text v-else depth="3">{{ $t('common.selectItem') }}</n-text>
    </div>
  </div>
</template>
<script setup lang="ts">
import { ref, computed } from 'vue'
import { NButton, NText } from 'naive-ui'
import DynamicFormField from '../../DynamicFormField.vue'
import DetailPreviewPanel from './DetailPreviewPanel.vue'
import {
  canAddArrayItem,
  canRemoveArrayItem,
  filterVisibleByContext,
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
const selectedIndex = ref(0)

/** 当前选中条目的动态可见字段（visibleWhen 按条目自身值过滤）。 */
const visibleSubFields = computed(() =>
  filterVisibleByContext(subFields.value, itemValue(selectedIndex.value)),
)

/** 读取指定条目的值对象；缺失时返回空对象。 */
function itemValue(idx: number): Record<string, unknown> {
  const item = listValue.value[idx]
  if (item && typeof item === 'object' && !Array.isArray(item)) {
    return item as Record<string, unknown>
  }
  return {}
}

const selectedParamValue = computed<string | undefined>(() => {
  const da = props.field.detailAction
  if (!da) return undefined
  const item = listValue.value[selectedIndex.value]
  if (!item || typeof item !== 'object' || Array.isArray(item)) return undefined
  const val = (item as Record<string, unknown>)[da.paramField]
  return typeof val === 'string' && val.length > 0 ? val : undefined
})

const listValue = computed<unknown[]>(() => {
  if (Array.isArray(props.modelValue)) return props.modelValue as unknown[]
  return []
})

/** 读取主从条目字段值；缺失值保持 undefined，由后端决定最终语义。 */
function getField(idx: number, key: string): unknown {
  const item = listValue.value[idx]
  if (item && typeof item === 'object' && !Array.isArray(item)) {
    return (item as Record<string, unknown>)[key]
  }
  return undefined
}

/** 更新主从条目字段并保持父级只读约束。 */
function setField(idx: number, key: string, val: unknown): void {
  if (props.field.readOnly) return
  const arr = [...listValue.value]
  const item = arr[idx]
  if (item && typeof item === 'object' && !Array.isArray(item)) {
    arr[idx] = { ...(item as Record<string, unknown>), [key]: val }
  }
  emit('update:modelValue', arr)
}

/** 生成左侧条目的摘要文本。 */
function summary(idx: number): string {
  const item = listValue.value[idx]
  if (!item || typeof item !== 'object' || Array.isArray(item)) return `#${idx + 1}`
  const firstField = subFields.value[0]
  if (!firstField) return `#${idx + 1}`
  return String((item as Record<string, unknown>)[firstField.key] ?? `#${idx + 1}`)
}

/** v-for 稳定 key：优先取行内首个字段值（如 name/id），避免索引 key 在中间删除时
 *  子字段组件（含内部输入态）串位；值可能重复/变更时退化为索引（与删除串位无关）。 */
function rowKey(item: unknown, idx: number): string {
  if (item && typeof item === 'object' && !Array.isArray(item)) {
    const firstKey = subFields.value[0]?.key
    if (firstKey !== undefined) {
      const v = (item as Record<string, unknown>)[firstKey]
      if (typeof v === 'string' || typeof v === 'number') return `${v}`
    }
  }
  return `idx-${idx}`
}

/** 添加一个符合 item schema 默认值的条目。 */
function onAdd(): void {
  if (!canAddArrayItem(props.field.schema, listValue.value.length) || props.field.readOnly || !itemSchema.value) return
  const arr = [...listValue.value]
  arr.push(getDefaultArrayItem(itemSchema.value))
  selectedIndex.value = arr.length - 1
  emit('update:modelValue', arr)
}

/** 删除条目并遵守 minItems 约束。 */
function onRemove(idx: number): void {
  if (!canRemoveArrayItem(props.field.schema, listValue.value.length) || props.field.readOnly) return
  const arr = [...listValue.value]
  arr.splice(idx, 1)
  selectedIndex.value = Math.min(selectedIndex.value, Math.max(0, arr.length - 1))
  emit('update:modelValue', arr)
}
</script>

<style scoped>
.array-master-detail {
  display: flex;
  gap: 12px;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  overflow: hidden;
  min-width: 0;
}
.md-list {
  width: 200px;
  flex-shrink: 0;
  border-right: 1px solid var(--border-color);
  padding: 8px;
  display: flex;
  flex-direction: column;
  gap: 4px;
  overflow-y: auto;
}
.md-list-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 8px;
  padding: 4px 8px;
  border-radius: 4px;
  cursor: pointer;
  font-size: var(--font-size-sm);
}
.md-list-item:hover {
  background-color: var(--hover-bg);
}
.md-list-item.active {
  background-color: var(--primary-color);
  color: white;
}
.md-detail {
  flex: 1;
  min-width: 0;
  padding: 12px;
  min-height: 0;
}
.md-list-item > span {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.md-list-item .n-button {
  flex-shrink: 0;
}
.md-field {
  margin-bottom: 8px;
}
</style>
