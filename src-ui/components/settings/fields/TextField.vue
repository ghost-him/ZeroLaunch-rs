<template>
  <div class="field-input-row">
    <n-input
      :value="modelValue as string"
      :disabled="!definition.field.editable"
      :placeholder="definition.field.description"
      @update:value="emit('update:modelValue', $event)"
    />
    <ConfigActionButton
      v-if="definition.configAction"
      :component-id="componentId"
      :config-action="definition.configAction"
      :field-key="definition.field.key"
      :editable="definition.field.editable"
      :model-value="modelValue"
      @update:model-value="emit('update:modelValue', $event)"
    />
  </div>
</template>

<script setup lang="ts">
import { NInput } from 'naive-ui'
import ConfigActionButton from '../ConfigActionButton.vue'
import type { SettingDefinition } from '../../../bridge/contract'

defineProps<{
  definition: SettingDefinition
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
.field-input-row > :first-child {
  flex: 1;
}
</style>
