<template>
  <div class="array-field">
    <n-alert v-if="unsupportedReason" type="warning">
      {{ $t('settings.unsupportedArray', { field: resolveText(field.label), reason: unsupportedReason }) }}
    </n-alert>

    <!-- 对象数组：卡片模式或默认列表模式 -->
    <ObjectCardsArray
      v-else-if="(uiKind === 'cards' || uiKind === 'list') && itemType === 'object'"
      :field="field"
      :component-id="componentId"
      :model-value="arrayValue"
      @update:model-value="emit('update:modelValue', $event)"
    />

    <!-- 对象数组：表格模式 -->
    <ObjectTableArray
      v-else-if="uiKind === 'table' && itemType === 'object'"
      :field="field"
      :component-id="componentId"
      :model-value="arrayValue"
      @update:model-value="emit('update:modelValue', $event)"
    />

    <!-- 对象数组：主从详情 -->
    <ObjectMasterDetailArray
      v-else-if="uiKind === 'masterDetail' && itemType === 'object'"
      :field="field"
      :component-id="componentId"
      :model-value="arrayValue"
      @update:model-value="emit('update:modelValue', $event)"
    />

    <!-- 搜索弹窗表格 -->
    <SearchTableArray
      v-else-if="uiKind === 'searchTable' && itemType === 'object'"
      :field="field"
      :component-id="componentId"
      :model-value="arrayValue"
      @update:model-value="emit('update:modelValue', $event)"
    />

    <!-- 原语数组：行列表（支持 string/number/boolean/enum） -->
    <PrimitiveRowList
      v-else-if="uiKind === 'list' && itemType === 'primitive'"
      :field="field"
      :model-value="arrayValue"
      :component-id="componentId"
      @update:model-value="emit('update:modelValue', $event)"
    />

    <!-- 原语数组：标签模式（仅纯字符串） -->
    <PrimitiveTagsArray
      v-else-if="uiKind === 'tags' && itemType === 'primitive'"
      :field="field"
      :model-value="arrayValue"
      @update:model-value="emit('update:modelValue', $event)"
    />

    <ConfigActionButton
      v-if="field.action && uiKind !== 'searchTable' && !unsupportedReason"
      :component-id="componentId"
      :field-action="field.action"
      :field-key="field.key"
      :editable="!field.readOnly"
      @update:model-value="emit('update:modelValue', $event)"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, watch } from 'vue'
import { NAlert } from 'naive-ui'
import { useI18n } from 'vue-i18n'
import { resolveText } from '../../../i18n'
import ConfigActionButton from '../ConfigActionButton.vue'
import PrimitiveRowList from './array/PrimitiveRowList.vue'
import PrimitiveTagsArray from './array/PrimitiveTagsArray.vue'
import ObjectCardsArray from './array/ObjectCardsArray.vue'
import ObjectTableArray from './array/ObjectTableArray.vue'
import ObjectMasterDetailArray from './array/ObjectMasterDetailArray.vue'
import SearchTableArray from './array/SearchTableArray.vue'
import {
  getArrayItemFieldConfig,
  getArrayItemKind,
  getArrayItemSchema,
  getPrimitiveArrayEditorKind,
  getSchemaTypeName,
  isObjectSchema,
  widgetToArrayUiKind,
} from '../../../utils/schemaTypes'
import type { FieldConfig } from '../../../utils/schemaTypes'

const props = defineProps<{
  field: FieldConfig
  componentId: string
  modelValue: unknown
}>()
const { t } = useI18n()

const emit = defineEmits<{
  (e: 'update:modelValue', value: unknown): void
}>()

/** 数组 UI 类型。 */
const uiKind = computed(() => widgetToArrayUiKind(props.field.widget))

/** 子项 schema；缺少 items 时保留为 null，让用户看到明确警告。 */
const itemSchema = computed(() => getArrayItemSchema(props.field.schema))
const itemKind = computed(() => getArrayItemKind(itemSchema.value))
const itemField = computed(() => getArrayItemFieldConfig(props.field))

/** 子项类型：object 或 primitive。 */
const itemType = computed(() => (itemKind.value === 'object' ? 'object' : 'primitive'))

/** 数组值。 */
const arrayValue = computed<unknown[]>(() => {
  if (Array.isArray(props.modelValue)) return props.modelValue
  return []
})

/** 检查 item schema 与数组 widget 组合是否可渲染。 */
const unsupportedReason = computed<string | null>(() => {
  if (props.field.detailAction && uiKind.value !== 'masterDetail') {
    return t('settings.detailActionUnsupportedArray')
  }
  if (!itemSchema.value) return t('settings.arrayItemsMissing')
  if (itemKind.value === 'unsupported') {
    return t('settings.arrayUnknownItemType', { type: getSchemaTypeName(itemSchema.value) })
  }
  switch (uiKind.value) {
    case 'tags':
      if (itemKind.value !== 'string') return t('settings.arrayTagsStringOnly')
      if (itemField.value?.widget?.kind !== 'text') {
        return t('settings.unsupportedWidget', {
          schemaType: getSchemaTypeName(itemSchema.value),
          widget: itemField.value?.widget?.kind ?? 'unknown',
        })
      }
      return null
    case 'table':
    case 'cards':
    case 'masterDetail':
      return isObjectSchema(itemSchema.value) ? null : t('settings.arrayObjectOnly', { widget: uiKind.value })
    case 'searchTable':
      return isObjectSchema(itemSchema.value) ? null : t('settings.arraySearchObjectOnly')
    case 'list':
      if (itemKind.value !== 'object' && getPrimitiveArrayEditorKind(props.field) === null) {
        return t('settings.unsupportedWidget', {
          schemaType: getSchemaTypeName(itemSchema.value),
          widget: itemField.value?.widget?.kind ?? 'unknown',
        })
      }
      return null
  }
})

/** 记录未实现的数组组合，避免错误落入只读 JSON 显示。 */
watch(unsupportedReason, (reason) => {
  if (reason) {
    console.warn(`[settings] ${props.field.key}: ${reason}`)
  }
}, { immediate: true })
</script>

<style scoped>
.array-field {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
</style>
