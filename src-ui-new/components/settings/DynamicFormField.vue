<template>
  <div class="form-field" v-if="definition.field.visible">
    <label class="field-label">
      {{ definition.field.label }}
      <span class="field-desc" v-if="definition.field.description"> — {{ definition.field.description }}</span>
    </label>

    <div class="field-control">
      <!-- Text -->
      <n-input
        v-if="isSimple('Text')"
        :value="modelValue as string"
        :disabled="!definition.field.editable"
        :placeholder="definition.field.description"
        @update:value="$emit('update:modelValue', $event)"
      />

      <!-- Number -->
      <n-input-number
        v-else-if="isNumber"
        :value="modelValue as number"
        :min="numberConfig.min"
        :max="numberConfig.max"
        :step="numberConfig.step"
        :disabled="!definition.field.editable"
        @update:value="$emit('update:modelValue', $event)"
      />

      <!-- Boolean -->
      <n-switch
        v-else-if="isSimple('Boolean')"
        :value="modelValue as boolean"
        :disabled="!definition.field.editable"
        @update:value="$emit('update:modelValue', $event)"
      />

      <!-- Select -->
      <n-select
        v-else-if="isSelect"
        :value="modelValue as string"
        :options="selectOptions"
        :disabled="!definition.field.editable"
        @update:value="$emit('update:modelValue', $event)"
      />

      <!-- Color -->
      <n-color-picker
        v-else-if="isSimple('Color')"
        :value="modelValue as string"
        :disabled="!definition.field.editable"
        @update:value="$emit('update:modelValue', $event)"
      />

      <!-- Json -->
      <n-input
        v-else-if="isSimple('Json')"
        type="textarea"
        :value="jsonString"
        :disabled="!definition.field.editable"
        :autosize="{ minRows: 2, maxRows: 6 }"
        @update:value="onJsonInput"
      />

      <!-- Config action attached to this field -->
      <ConfigActionButton
        v-if="definition.configAction"
        :component-id="componentId"
        :field="definition.field"
        :model-value="modelValue"
        @update:model-value="$emit('update:modelValue', $event)"
      />

      <!-- Unknown / unsupported type fallback -->
      <n-input
        v-else
        :value="JSON.stringify(modelValue)"
        disabled
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { NInput, NInputNumber, NSwitch, NSelect, NColorPicker } from 'naive-ui'
import ConfigActionButton from './ConfigActionButton.vue'
import type { FieldDefinition } from '../../bridge/contract'

const props = defineProps<{
  definition: {
    field: FieldDefinition
    group?: string
    order: number
    configAction?: string
  }
  componentId: string
  modelValue: unknown
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: unknown): void
}>()

function isSimple(type: string): boolean {
  return props.definition.field.settingType === type
}

const isNumber = computed(() => {
  const st = props.definition.field.settingType
  return typeof st === 'object' && 'Number' in st
})

const numberConfig = computed(() => {
  const st = props.definition.field.settingType
  if (typeof st === 'object' && 'Number' in st) {
    return st.Number
  }
  return { min: undefined, max: undefined, step: undefined }
})

const isSelect = computed(() => {
  const st = props.definition.field.settingType
  return typeof st === 'object' && 'Select' in st
})

const selectOptions = computed(() => {
  const st = props.definition.field.settingType
  if (typeof st === 'object' && 'Select' in st) {
    return st.Select.options.map((o) => ({ label: o, value: o }))
  }
  return []
})

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
.form-field {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.field-label {
  font-size: var(--font-size-sm);
  color: var(--text-primary);
}

.field-desc {
  color: var(--text-secondary);
  font-weight: 400;
}

.field-control {
  display: flex;
  gap: 8px;
  align-items: center;
}
</style>
