<template>
  <div class="primitive-row-list">
    <div v-for="(item, idx) in listValue" :key="idx" class="array-row">
      <PathField
        v-if="editorKind === 'path' && itemField"
        :field="itemField"
        :component-id="componentId"
        :model-value="item"
        @update:model-value="(val: unknown) => onUpdate(idx, val)"
      />
      <ColorField
        v-else-if="editorKind === 'color' && itemField"
        :field="itemField"
        :component-id="componentId"
        :model-value="item"
        @update:model-value="(val: unknown) => onUpdate(idx, val)"
      />
      <n-input
        v-else-if="editorKind === 'text'"
        :value="item as string"
        :disabled="field.readOnly"
        size="small"
        @update:value="(val: string) => onUpdate(idx, val)"
      />
      <n-select
        v-else-if="editorKind === 'select'"
        :value="item as string"
        :options="enumOptions"
        :disabled="field.readOnly"
        size="small"
        @update:value="(val: string) => onUpdate(idx, val)"
      />
      <n-input-number
        v-else-if="editorKind === 'number'"
        :value="item as number"
        :min="numConfig.min ?? undefined"
        :max="numConfig.max ?? undefined"
        :step="numConfig.step ?? undefined"
        :disabled="field.readOnly"
        size="small"
        @update:value="(val: number | null) => onUpdate(idx, val ?? 0)"
      />
      <n-switch
        v-else-if="editorKind === 'boolean'"
        :value="item as boolean"
        :disabled="field.readOnly"
        @update:value="(val: boolean) => onUpdate(idx, val)"
      />
      <n-button
        text type="error" size="tiny"
        :disabled="field.readOnly || !canRemoveArrayItem(field.schema, listValue.length)"
        @click="onRemove(idx)"
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
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { NInput, NInputNumber, NSelect, NSwitch, NButton } from 'naive-ui'
import PathField from '../PathField.vue'
import ColorField from '../ColorField.vue'
import {
  canAddArrayItem,
  canRemoveArrayItem,
  getArrayItemFieldConfig,
  getDefaultArrayItem,
  getPrimitiveArrayEditorKind,
  getSchemaEnumOptions,
  getSchemaNumberConfig,
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

/** 解析数组元素字段配置，统一交给 schema 工具处理类型和 widget。 */
const itemField = computed(() => getArrayItemFieldConfig(props.field))

/** 根据 itemWidget 选择数组元素编辑器。 */
const editorKind = computed(() => getPrimitiveArrayEditorKind(props.field))

/** 数组值，非法输入按空数组展示并交由后端最终校验。 */
const listValue = computed<readonly unknown[]>(() => {
  if (Array.isArray(props.modelValue)) return props.modelValue
  return []
})

/** 数值配置（number / integer 专用）。 */
const numConfig = computed(() => itemField.value ? getSchemaNumberConfig(itemField.value.schema) : { min: null, max: null, step: null })

/** 枚举选项（select 专用）。 */
const enumOptions = computed(() => itemField.value ? getSchemaEnumOptions(itemField.value.schema) : [])

/** 添加一个由后端明确 default 或空值表示的数组元素。 */
function onAdd(): void {
  if (!canAddArrayItem(props.field.schema, listValue.value.length) || props.field.readOnly || !itemField.value) return
  const arr = [...listValue.value]
  arr.push(getDefaultArrayItem(itemField.value.schema))
  emit('update:modelValue', arr)
}

/** 删除原语并遵守 minItems 约束。 */
function onRemove(idx: number): void {
  if (!canRemoveArrayItem(props.field.schema, listValue.value.length) || props.field.readOnly) return
  const arr = [...listValue.value]
  arr.splice(idx, 1)
  emit('update:modelValue', arr)
}

/** 更新原语值并保持数组引用不可变。 */
function onUpdate(idx: number, val: unknown): void {
  if (props.field.readOnly) return
  const arr = [...listValue.value]
  arr[idx] = val
  emit('update:modelValue', arr)
}
</script>

<style scoped>
.primitive-row-list {
  display: flex;
  flex-direction: column;
  gap: var(--gap-sm, 4px);
}
.array-row {
  display: flex;
  gap: var(--gap-sm, 4px);
  align-items: center;
}
</style>
