<template>
  <div class="field-input-row">
    <n-input
      :type="inputType"
      :value="modelValue as string"
      :disabled="field.readOnly"
      :placeholder="resolveText(field.description)"
      :minlength="stringConfig.minLength ?? undefined"
      :maxlength="stringConfig.maxLength ?? undefined"
      :input-props="stringConfig.pattern ? { pattern: stringConfig.pattern } : undefined"
      :rows="inputKind === 'textarea' ? 6 : undefined"
      :show-password-on="field.sensitive ? 'click' : undefined"
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
/**
 * 输入框类型：敏感字段用密码框，其余按 widget 决定。
 * 只有渲染方式变化，绑定值始终是唯一真值，不会把遮罩文本回写给表单。
 */
const inputType = computed<'text' | 'password' | 'textarea'>(() =>
  props.field.sensitive ? 'password' : inputKind.value,
)

const emit = defineEmits<{
  (e: 'update:modelValue', value: unknown): void
}>()
</script>

<style scoped>
.field-input-row {
  display: flex;
  gap: 8px;
  align-items: center;
  min-width: 0;
}
.field-input-row > :first-child {
  flex: 1;
  min-width: 0;
}
</style>
