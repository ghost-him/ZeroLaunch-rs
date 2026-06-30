<template>
  <div class="field-input-row">
    <n-input-number
      :value="modelValue as number"
      :min="config.min"
      :max="config.max"
      :step="config.step"
      :disabled="!definition.field.editable"
      @update:value="(val: number | null) => emit('update:modelValue', val ?? 0)"
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
import { computed } from 'vue'
import { NInputNumber } from 'naive-ui'
import ConfigActionButton from '../ConfigActionButton.vue'
import { getNumberConfig } from '../../../utils/schemaTypes'
import type { SettingDefinition } from '../../../bridge/contract'

const props = defineProps<{
  definition: SettingDefinition
  componentId: string
  modelValue: unknown
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: unknown): void
}>()

const config = computed(() => getNumberConfig(props.definition.field.settingType))
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
