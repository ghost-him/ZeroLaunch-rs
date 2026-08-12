<template>
  <div class="font-field">
    <n-select
      :value="modelValue as string"
      :options="options"
      :loading="loading"
      :disabled="field.readOnly"
      :placeholder="t('settings.fontFollowSystem')"
      :render-label="renderLabel"
      filterable
      clearable
      @update:value="emit('update:modelValue', $event)"
    />
    <div v-if="loadFailed" class="font-field-error">
      <span class="font-field-error-text">{{ t('settings.fontLoadFailed') }}</span>
      <n-button size="tiny" quaternary type="primary" @click="loadFonts">
        {{ t('settings.retry') }}
      </n-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, h, onMounted, ref } from 'vue'
import { NButton, NSelect } from 'naive-ui'
import type { SelectOption } from 'naive-ui'
import { useI18n } from 'vue-i18n'
import { useConfigStore } from '../../../stores/config-store'
import { getSchemaFontSource } from '../../../utils/schemaTypes'
import type { FieldConfig } from '../../../utils/schemaTypes'

const props = defineProps<{
  field: FieldConfig
  componentId: string
  modelValue: unknown
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: unknown): void
}>()

const { t } = useI18n()
const configStore = useConfigStore()

const fonts = ref<string[]>([])
const loading = ref(false)
const loadFailed = ref(false)

/** 字体列表来源（widget 声明的 config action），由 schemaTypes 统一收窄。 */
const fontSource = computed(() => getSchemaFontSource(props.field.widget))

/** 候选列表：首项为"跟随系统"（空值），其余为系统已安装字体。 */
const options = computed(() => [
  { label: t('settings.fontFollowSystem'), value: '' },
  ...fonts.value.map((font) => ({ label: font, value: font })),
])

/** 以字体自身渲染选项标签，实现所见即所得的字体预览。 */
function renderLabel(option: SelectOption) {
  const value = String(option.value)
  const label = typeof option.label === 'string' ? option.label : value
  return value
    ? h('span', { style: { fontFamily: `'${value}', sans-serif` } }, label)
    : label
}

/** 通过组件声明的 config action 拉取系统字体列表。 */
async function loadFonts(): Promise<void> {
  const source = fontSource.value
  if (!source) return
  loading.value = true
  loadFailed.value = false
  try {
    const result = await configStore.executeAction(
      source.component ?? props.componentId,
      source.action,
    )
    const list = (result as { fonts?: unknown })?.fonts
    fonts.value = Array.isArray(list) ? (list as string[]) : []
  } catch {
    loadFailed.value = true
  } finally {
    loading.value = false
  }
}

onMounted(loadFonts)
</script>

<style scoped>
.font-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.font-field-error {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: var(--font-size-sm);
}
.font-field-error-text {
  color: var(--text-secondary);
}
</style>
