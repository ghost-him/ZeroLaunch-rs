<template>
  <n-dynamic-tags
    :value="modelValue as string[]"
    :disabled="field.readOnly"
    :max="arrayConfig.maxItems ?? undefined"
    @update:value="onUpdate"
  />
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { NDynamicTags } from 'naive-ui'
import {
  canAddArrayItem,
  canRemoveArrayItem,
  getArrayConstraints,
} from '../../../../utils/schemaTypes'
import type { FieldConfig } from '../../../../utils/schemaTypes'

const props = defineProps<{
  field: FieldConfig
  modelValue: unknown
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: unknown): void
}>()

const arrayConfig = computed(() => getArrayConstraints(props.field.schema))

/** 更新 tags 值并阻止越过 minItems/maxItems 约束。 */
function onUpdate(value: string[]): void {
  if (props.field.readOnly) return
  if (!canAddArrayItem(props.field.schema, value.length)) return
  if (!canRemoveArrayItem(props.field.schema, value.length)) return
  emit('update:modelValue', value)
}
</script>
