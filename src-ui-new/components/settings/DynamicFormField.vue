<template>
  <div class="form-field" v-if="definition.field.visible">
    <label class="field-label">
      {{ definition.field.label }}
      <span class="field-desc" v-if="definition.field.description">
        — {{ definition.field.description }}
      </span>
    </label>

    <div class="field-control">
      <TextField
        v-if="isTextType(st)"
        :definition="definition"
        :component-id="componentId"
        :model-value="modelValue"
        @update:model-value="emit('update:modelValue', $event)"
      />
      <NumberField
        v-else-if="isNumberType(st)"
        :definition="definition"
        :component-id="componentId"
        :model-value="modelValue"
        @update:model-value="emit('update:modelValue', $event)"
      />
      <BooleanField
        v-else-if="isBooleanType(st)"
        :definition="definition"
        :component-id="componentId"
        :model-value="modelValue"
        @update:model-value="emit('update:modelValue', $event)"
      />
      <SelectField
        v-else-if="isSelectType(st)"
        :definition="definition"
        :component-id="componentId"
        :model-value="modelValue"
        @update:model-value="emit('update:modelValue', $event)"
      />
      <ColorField
        v-else-if="isColorType(st)"
        :definition="definition"
        :component-id="componentId"
        :model-value="modelValue"
        @update:model-value="emit('update:modelValue', $event)"
      />
      <PathField
        v-else-if="isPathType(st)"
        :definition="definition"
        :component-id="componentId"
        :model-value="modelValue"
        @update:model-value="emit('update:modelValue', $event)"
      />
      <JsonField
        v-else-if="isJsonType(st)"
        :definition="definition"
        :component-id="componentId"
        :model-value="modelValue"
        @update:model-value="emit('update:modelValue', $event)"
      />
      <ArrayField
        v-else-if="isArrayType(st)"
        :definition="definition"
        :component-id="componentId"
        :model-value="modelValue"
        @update:model-value="emit('update:modelValue', $event)"
      />
      <ImageField
        v-else-if="isImageType(st)"
        :definition="definition"
        :component-id="componentId"
        :model-value="modelValue"
        @update:model-value="emit('update:modelValue', $event)"
      />
      <n-input v-else :value="JSON.stringify(modelValue)" disabled />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { NInput } from 'naive-ui'
import {
  isTextType,
  isNumberType,
  isBooleanType,
  isSelectType,
  isColorType,
  isPathType,
  isJsonType,
  isArrayType,
  isImageType,
} from '../../utils/schemaTypes'
import type { SettingDefinition, SettingType } from '../../bridge/contract'
import TextField from './fields/TextField.vue'
import NumberField from './fields/NumberField.vue'
import BooleanField from './fields/BooleanField.vue'
import SelectField from './fields/SelectField.vue'
import ColorField from './fields/ColorField.vue'
import PathField from './fields/PathField.vue'
import JsonField from './fields/JsonField.vue'
import ArrayField from './fields/ArrayField.vue'
import ImageField from './fields/ImageField.vue'

const props = defineProps<{
  definition: SettingDefinition
  componentId: string
  modelValue: unknown
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: unknown): void
}>()

const st = computed<SettingType>(() => props.definition.field.settingType)
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
  flex-direction: column;
}
</style>
