<template>
  <div class="array-field">
    <ConfigActionButton
      v-if="definition.configAction"
      :component-id="componentId"
      :config-action="definition.configAction"
      :field-key="definition.field.key"
      :editable="definition.field.editable"
      :model-value="modelValue"
      @update:model-value="emit('update:modelValue', $event)"
    />

    <PrimitiveTagsArray
      v-if="isPrimitive && uiHint === 'Tags'"
      :definition="definition"
      :model-value="arrayValue"
      :item="arrayItem"
      @update:model-value="emit('update:modelValue', $event)"
    />

    <PrimitiveListArray
      v-else-if="isPrimitive"
      :definition="definition"
      :model-value="arrayValue"
      :item="arrayItem"
      @update:model-value="emit('update:modelValue', $event)"
    />

    <ObjectMasterDetailArray
      v-else-if="isObject && uiHint === 'MasterDetail'"
      :definition="definition"
      :component-id="componentId"
      :model-value="arrayValue"
      :item="arrayItem"
      @update:model-value="emit('update:modelValue', $event)"
    />

    <ObjectTableArray
      v-else-if="isObject && uiHint === 'Table'"
      :definition="definition"
      :model-value="arrayValue"
      :item="arrayItem"
      @update:model-value="emit('update:modelValue', $event)"
    />

    <ObjectCardsArray
      v-else-if="isObject"
      :definition="definition"
      :component-id="componentId"
      :model-value="arrayValue"
      :item="arrayItem"
      @update:model-value="emit('update:modelValue', $event)"
    />

    <n-text v-else depth="3">不支持的数组配置</n-text>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { NText } from 'naive-ui'
import ConfigActionButton from '../ConfigActionButton.vue'
import { isPrimitiveArray, isObjectArray } from '../../../utils/schemaTypes'
import type { SettingDefinition, ArrayItem, ArrayUiHint } from '../../../bridge/contract'
import PrimitiveTagsArray from './array/PrimitiveTagsArray.vue'
import PrimitiveListArray from './array/PrimitiveListArray.vue'
import ObjectCardsArray from './array/ObjectCardsArray.vue'
import ObjectTableArray from './array/ObjectTableArray.vue'
import ObjectMasterDetailArray from './array/ObjectMasterDetailArray.vue'

const props = defineProps<{
  definition: SettingDefinition
  componentId: string
  modelValue: unknown
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: unknown): void
}>()

const arrayItem = computed<ArrayItem>(() => {
  const st = props.definition.field.settingType
  if (typeof st === 'object' && st !== null && 'Array' in st) {
    return st.Array.item
  }
  return { Primitive: 'Text' }
})

const uiHint = computed<ArrayUiHint>(() => {
  const st = props.definition.field.settingType
  if (typeof st === 'object' && st !== null && 'Array' in st) {
    return st.Array.uiHint
  }
  return 'Default'
})

const isPrimitive = computed(() => isPrimitiveArray(arrayItem.value))
const isObject = computed(() => isObjectArray(arrayItem.value))

const arrayValue = computed<unknown[]>(() => {
  if (Array.isArray(props.modelValue)) return props.modelValue as unknown[]
  return []
})
</script>

<style scoped>
.array-field {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
</style>
