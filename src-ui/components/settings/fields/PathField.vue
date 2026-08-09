<template>
  <div class="field-input-row">
    <n-button :disabled="field.readOnly" @click="openPicker">{{ resolveText(field.label) }}</n-button>
    <span class="path-display" v-if="modelValue">{{ modelValue as string }}</span>
    <span class="path-display path-placeholder" v-else>{{ $t('settings.pathNotSelected') }}</span>
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
import { NButton } from 'naive-ui'
import { resolveText } from '../../../i18n'
import ConfigActionButton from '../ConfigActionButton.vue'
import { getSchemaPathMode } from '../../../utils/schemaTypes'
import type { FieldConfig } from '../../../utils/schemaTypes'

const props = defineProps<{
  field: FieldConfig
  componentId: string
  modelValue: unknown
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: unknown): void
}>()

const mode = computed(() => getSchemaPathMode(props.field.widget))

/** 直接调用 Tauri dialog 仅负责选择路径并回传值，不承载业务逻辑。 */
async function openPicker() {
  if (props.field.readOnly) return
  try {
    const { open } = await import('@tauri-apps/plugin-dialog')
    const selected = await open({
      directory: mode.value === 'directory',
      multiple: false,
    })
    if (selected) {
      emit('update:modelValue', selected)
    }
  } catch {
    // Fallback for non-Tauri environment
  }
}
</script>

<style scoped>
.field-input-row {
  display: flex;
  gap: 8px;
  align-items: center;
}
.path-display {
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
  max-width: 300px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.path-placeholder {
  color: var(--text-disabled);
}
</style>
