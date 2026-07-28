<template>
  <div class="field-input-row">
    <div class="image-preview" v-if="modelValue">
      <n-image
        :src="imageSrc"
        :preview-disabled="!editable"
        :alt="field.label"
        object-fit="cover"
        height="64"
        width="64"
      />
      <n-button
        v-if="editable"
        text
        type="error"
        size="tiny"
        @click="emit('update:modelValue', null)"
      >
        {{ $t('settings.imageClear') }}
      </n-button>
    </div>
    <n-button
      v-else
      :disabled="!editable"
      @click="uploadImage"
    >
      {{ $t('settings.imageSelect') }}
    </n-button>
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
import { computed, ref, watch } from 'vue'
import { NButton, NImage, useMessage } from 'naive-ui'
import { useI18n } from 'vue-i18n'
import { resourceGet, resourceUpload } from '../../../bridge/commands'
import ConfigActionButton from '../ConfigActionButton.vue'
import { getSchemaImageConfig } from '../../../utils/schemaTypes'
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
const imageConfig = computed(() => getSchemaImageConfig(props.field.widget))
const editable = computed(() => !props.field.readOnly)

const imageSrc = ref('')

watch(() => props.modelValue, async (val) => {
  if (!val) { imageSrc.value = ''; return }
  const rid = String(val)
  if (rid.startsWith('http://') || rid.startsWith('https://')) { imageSrc.value = rid; return }
  if (rid.startsWith('data:')) { imageSrc.value = rid; return }
  try {
    imageSrc.value = await resourceGet(rid)
  } catch {
    imageSrc.value = ''
  }
}, { immediate: true })

/** 直接调用 Tauri dialog 选择文件，再通过 IPC 上传资源，不承载业务逻辑。 */
async function uploadImage() {
  if (!editable.value) return
  try {
    const { open } = await import('@tauri-apps/plugin-dialog')
    const selected = await open({
      multiple: false,
      filters: [{ name: props.field.label, extensions: imageConfig.value.accept }],
    })
    if (selected) {
      const rid = await resourceUpload(selected, props.field.key, imageConfig.value.maxSize ?? undefined)
      emit('update:modelValue', rid)
      message.success(t('settings.imageUploadSuccess'))
    }
  } catch {
    message.error(t('settings.imageUploadFailed'))
  }
}
</script>

<style scoped>
.field-input-row {
  display: flex;
  gap: 8px;
  align-items: center;
}
.image-preview {
  display: flex;
  align-items: center;
  gap: 8px;
}
</style>
