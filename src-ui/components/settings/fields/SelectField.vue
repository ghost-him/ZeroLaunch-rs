<template>
  <div class="field-input-row">
    <n-select
      :value="modelValue as string"
      :options="options"
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
import { computed } from 'vue'
import { NSelect } from 'naive-ui'
import ConfigActionButton from '../ConfigActionButton.vue'
import { getSelectOptions } from '../../../utils/schemaTypes'
import type { SettingDefinition } from '../../../bridge/contract'

const props = defineProps<{
  definition: SettingDefinition
  componentId: string
  modelValue: unknown
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: unknown): void
}>()

const options = computed(() => getSelectOptions(props.definition.field.settingType))
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
