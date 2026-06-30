<template>
  <div class="json-field">
    <div class="field-input-row">
      <n-input
        type="textarea"
        :value="jsonString"
        :disabled="!definition.field.editable"
        :autosize="{ minRows: 2, maxRows: 6 }"
        @update:value="onJsonInput"
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
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { NInput } from 'naive-ui'
import ConfigActionButton from '../ConfigActionButton.vue'
import type { SettingDefinition } from '../../../bridge/contract'

const props = defineProps<{
  definition: SettingDefinition
  componentId: string
  modelValue: unknown
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: unknown): void
}>()

const jsonString = computed(() => {
  if (props.modelValue == null) return ''
  try {
    return JSON.stringify(props.modelValue, null, 2)
  } catch {
    return String(props.modelValue)
  }
})

function onJsonInput(val: string) {
  try {
    emit('update:modelValue', JSON.parse(val))
  } catch {
    // Invalid JSON, silently ignore
  }
}
</script>

<style scoped>
.json-field {
  display: flex;
  flex-direction: column;
}
.field-input-row {
  display: flex;
  gap: 8px;
  align-items: flex-start;
}
.field-input-row > :first-child {
  flex: 1;
}
</style>
