<template>
  <div ref="rootEl" class="field-input-row" :class="{ 'drop-hover': isHovered }">
    <n-button :disabled="field.readOnly" @click="openPicker">{{ resolveText(field.label) }}</n-button>
    <span class="path-display" v-if="modelValue">{{ modelValue as string }}</span>
    <span class="path-display path-placeholder" v-else>{{ $t('settings.pathNotSelected') }}</span>
    <span v-if="isHovered" class="drop-hint">{{ $t('settings.drop.usePath') }}</span>
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
import { computed, ref } from 'vue'
import { NButton, useMessage } from 'naive-ui'
import { useI18n } from 'vue-i18n'
import { resolveText } from '../../../i18n'
import ConfigActionButton from '../ConfigActionButton.vue'
import { getSchemaPathMode } from '../../../utils/schemaTypes'
import { useFileDropTarget } from '../../../composables/useFileDrop'
import type { FieldConfig } from '../../../utils/schemaTypes'

const props = defineProps<{
  field: FieldConfig
  componentId: string
  modelValue: unknown
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: unknown): void
}>()

const message = useMessage()
const { t } = useI18n()
const rootEl = ref<HTMLElement | null>(null)
const mode = computed(() => getSchemaPathMode(props.field.widget))

/** 拖放投放：只接受与字段 mode 一致的对象（文件或目录），命中即覆盖当前值。 */
const { isHovered } = useFileDropTarget(rootEl, {
  enabled: !props.field.readOnly,
  onDrop: (paths) => {
    const wantDirectory = mode.value === 'directory'
    const accepted = paths.find((entry) =>
      wantDirectory ? entry.kind === 'directory' : entry.kind === 'file',
    )
    if (!accepted) {
      message.error(t('settings.drop.kindMismatch', {
        kind: wantDirectory ? t('settings.drop.directory') : t('settings.drop.file'),
      }))
      return
    }
    emit('update:modelValue', accepted.path)
  },
})

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
  min-width: 0;
}
.field-input-row > .n-button {
  flex-shrink: 0;
}
.path-display {
  flex: 1;
  min-width: 0;
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
.field-input-row.drop-hover {
  border-radius: 4px;
  outline: 1px dashed var(--primary-color);
  outline-offset: 2px;
}
.drop-hint {
  flex-shrink: 0;
  font-size: var(--font-size-sm);
  color: var(--primary-color);
}
</style>
