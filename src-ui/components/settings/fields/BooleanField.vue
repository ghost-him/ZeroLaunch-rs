<template>
  <div class="field-input-row">
    <n-switch
      :value="modelValue as boolean"
      :disabled="field.readOnly"
      @update:value="emit('update:modelValue', $event)"
    />
    <ConfigActionButton
      v-if="field.action"
      :component-id="componentId"
      :field-action="field.action"
      :field-key="field.key"
      :editable="!field.readOnly"
      @update:model-value="emit('update:modelValue', $event)"
    />
  </div>
</template>

<script setup lang="ts">
import { NSwitch } from 'naive-ui'
import ConfigActionButton from '../ConfigActionButton.vue'
import type { FieldConfig } from '../../../utils/schemaTypes'

defineProps<{
  field: FieldConfig
  componentId: string
  modelValue: unknown
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: unknown): void
}>()
</script>

<style scoped>
.field-input-row {
  display: flex;
  gap: 8px;
  align-items: center;
}
</style>
