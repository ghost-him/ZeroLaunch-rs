<template>
  <n-dynamic-tags
    :value="modelValue as string[]"
    :disabled="!definition.field.editable"
    @update:value="emit('update:modelValue', $event)"
  />
</template>

<script setup lang="ts">
import { NDynamicTags } from 'naive-ui'
import type { SettingDefinition, ArrayItem } from '../../../../bridge/contract'

defineProps<{
  definition: SettingDefinition
  modelValue: unknown
  item: ArrayItem
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: unknown): void
}>()
</script>
