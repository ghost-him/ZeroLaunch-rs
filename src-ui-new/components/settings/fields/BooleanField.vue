<template>
  <div class="field-input-row">
    <n-switch
      :value="modelValue as boolean"
      :disabled="!definition.field.editable"
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
import { NSwitch } from 'naive-ui'
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
</style>
