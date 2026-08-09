<template>
  <div class="field-input-row">
    <n-input
      :type="inputKind"
      :value="modelValue as string"
      :disabled="field.readOnly"
      :placeholder="resolveText(field.description)"
      :minlength="stringConfig.minLength ?? undefined"
      :maxlength="stringConfig.maxLength ?? undefined"
      :input-props="stringConfig.pattern ? { pattern: stringConfig.pattern } : undefined"
      :rows="inputKind === 'textarea' ? 6 : undefined"
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
import { computed } from 'vue'
import { NInput } from 'naive-ui'
import { resolveText } from '../../../i18n'
import ConfigActionButton from '../ConfigActionButton.vue'
import { getSchemaStringConfig, getTextInputKind } from '../../../utils/schemaTypes'
import type { FieldConfig } from '../../../utils/schemaTypes'

const props = defineProps<{
  field: FieldConfig
  componentId: string
  modelValue: unknown
}>()

const stringConfig = computed(() => getSchemaStringConfig(props.field.schema))
const inputKind = computed(() => getTextInputKind(props.field.widget))

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
