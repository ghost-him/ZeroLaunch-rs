<template>
  <div class="image-field">
    <div class="image-preview" v-if="previewUrl">
      <n-image
        :src="previewUrl"
        :width="160"
        :height="120"
        object-fit="cover"
        show-toolbar-tooltip
      />
    </div>
    <div class="image-actions">
      <n-button
        size="small"
        :disabled="!definition.field.editable"
        @click="selectImage"
      >
        {{ modelValue ? $t('settings.imageReselect') : $t('settings.imageSelect') }}
      </n-button>
      <n-button
        v-if="modelValue"
        size="small"
        :disabled="!definition.field.editable"
        @click="clearImage"
      >
        {{ $t('settings.imageClear') }}
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
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { NImage, NButton, useMessage } from 'naive-ui'
import { open } from '@tauri-apps/plugin-dialog'
import ConfigActionButton from '../ConfigActionButton.vue'
import { getImageConfig } from '../../../utils/schemaTypes'
import { resourceGet, resourceUpload } from '../../../bridge/commands'
import type { SettingDefinition } from '../../../bridge/contract'

const props = defineProps<{
  definition: SettingDefinition
  componentId: string
  modelValue: unknown
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: unknown): void
}>()

const { t } = useI18n()
const message = useMessage()
const config = getImageConfig(props.definition.field.settingType)
const previewUrl = ref<string>('')

async function loadPreview() {
  const val = props.modelValue
  if (typeof val !== 'string' || val.length === 0) {
    previewUrl.value = ''
    return
  }
  try {
    previewUrl.value = await resourceGet(val)
  } catch (e) {
    console.error('[ImageField] Failed to load preview:', e)
    message.error(t('settings.imageLoadFailed'))
    previewUrl.value = ''
  }
}

watch(() => props.modelValue, loadPreview, { immediate: true })

async function selectImage() {
  try {
    const selected = await open({
      multiple: false,
      filters: [
        {
          name: 'Images',
          extensions: config.accept,
        },
      ],
    })
    if (!selected || typeof selected !== 'string') return

    const resId = await resourceUpload(selected, props.definition.field.key, config.maxSize)
    emit('update:modelValue', resId)
  } catch (e) {
    console.error('[ImageField] Upload failed:', e)
    message.error(t('settings.imageUploadFailed'))
  }
}

function clearImage() {
  emit('update:modelValue', '')
}
</script>

<style scoped>
.image-field {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.image-preview {
  border-radius: 6px;
  overflow: hidden;
  border: 1px solid var(--border-color);
}

.image-actions {
  display: flex;
  gap: 8px;
  align-items: center;
}
</style>
