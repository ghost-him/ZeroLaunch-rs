<template>
  <div class="field-input-row">
    <n-input-number
      :value="modelValue as number"
      :min="config.min ?? undefined"
      :max="config.max ?? undefined"
      :step="config.step ?? undefined"
      :precision="config.precision"
      :disabled="field.readOnly"
      @update:value="(val: number | null) => emit('update:modelValue', val ?? 0)"
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
import { computed } from 'vue'
import { NInputNumber } from 'naive-ui'
import ConfigActionButton from '../ConfigActionButton.vue'
import { getSchemaNumberConfig } from '../../../utils/schemaTypes'
import type { FieldConfig } from '../../../utils/schemaTypes'

const props = defineProps<{
  field: FieldConfig
  componentId: string
  modelValue: unknown
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: unknown): void
}>()

const config = computed(() => getSchemaNumberConfig(props.field.schema))
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
