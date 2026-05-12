<template>
  <div class="field-input-row">
    <div class="path-input-row">
      <n-input
        :value="modelValue as string"
        :disabled="!definition.field.editable"
        :placeholder="placeholder"
        @update:value="emit('update:modelValue', $event)"
      />
      <n-button
        size="small"
        :disabled="!definition.field.editable"
        @click="browsePath"
      >
        浏览
      </n-button>
    </div>
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
import { NInput, NButton } from 'naive-ui'
import { open } from '@tauri-apps/plugin-dialog'
import ConfigActionButton from '../ConfigActionButton.vue'
import { getPathMode } from '../../../utils/schemaTypes'
import type { SettingDefinition } from '../../../bridge/contract'

const props = defineProps<{
  definition: SettingDefinition
  componentId: string
  modelValue: unknown
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: unknown): void
}>()

const mode = computed(() => getPathMode(props.definition.field.settingType))

const placeholder = computed(() =>
  mode.value === 'Directory' ? '选择目录...' : '选择文件...',
)

async function browsePath() {
  try {
    if (mode.value === 'Directory') {
      const selected = await open({ directory: true, multiple: false })
      if (selected && typeof selected === 'string') {
        emit('update:modelValue', selected)
      }
    } else {
      const selected = await open({ multiple: false })
      if (selected && typeof selected === 'string') {
        emit('update:modelValue', selected)
      }
    }
  } catch (e) {
    console.error('[PathField] Browse failed:', e)
  }
}
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
.path-input-row {
  display: flex;
  gap: 8px;
  align-items: center;
  flex: 1;
}
.path-input-row > :deep(.n-input) {
  flex: 1;
}
</style>
