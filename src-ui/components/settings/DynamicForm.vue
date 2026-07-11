<template>
  <div class="dynamic-form">
    <div class="form-header">
      <h3>{{ schema.componentName }}</h3>
      <n-tag :bordered="false" size="small">{{ schema.componentType }}</n-tag>
    </div>
    <p class="form-desc" v-if="schema.componentDescription">{{ schema.componentDescription }}</p>

    <div class="form-groups">
      <FormSection
        v-for="group in groupedSettings"
        :key="group.name"
        :title="group.name"
        :collapsible="group.name !== ''"
      >
        <DynamicFormField
          v-for="def in group.items"
          :key="def.field.key"
          :definition="def"
          :component-id="schema.componentId"
          :model-value="getValue(def.field.key)"
          @update:model-value="(val: unknown) => setValue(def.field.key, val)"
        />
      </FormSection>
    </div>

    <div class="form-actions">
      <n-button type="primary" :loading="saving" @click="onApply">应用</n-button>
      <n-button :loading="resetting" @click="onReset">重置为默认</n-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, provide } from 'vue'
import { NButton, NTag, useMessage, useDialog } from 'naive-ui'
import DynamicFormField from './DynamicFormField.vue'
import FormSection from './FormSection.vue'
import { useConfigStore } from '../../stores/config-store'
import { FORM_VALUES_KEY } from '../../utils/formInjection'
import type { ComponentSchema, SettingDefinition } from '../../bridge/contract'

const props = defineProps<{
  schema: ComponentSchema
  currentSettings: Record<string, unknown>
}>()

const emit = defineEmits<{
  (e: 'reload'): void
}>()

const message = useMessage()
const dialog = useDialog()
const configStore = useConfigStore()
const saving = ref(false)
const resetting = ref(false)
const localValues = ref<Record<string, unknown>>({ ...props.currentSettings })

watch(
  () => props.currentSettings,
  (newSettings) => {
    localValues.value = { ...newSettings }
  },
)

const groupedSettings = computed(() => {
  const groups = new Map<string, SettingDefinition[]>()
  for (const def of props.schema.settings) {
    if (def.field.visible === false) continue
    const g = def.group || ''
    if (!groups.has(g)) groups.set(g, [])
    groups.get(g)!.push(def)
  }
  for (const [, items] of groups) {
    items.sort((a, b) => a.order - b.order)
  }
  return [...groups.entries()].map(([name, items]) => ({ name, items }))
})

function getValue(key: string): unknown {
  return localValues.value[key]
}

function setValue(key: string, val: unknown) {
  localValues.value = { ...localValues.value, [key]: val }
}

provide(FORM_VALUES_KEY, { getValue, setValue, values: localValues })

async function onApply() {
  saving.value = true
  try {
    await configStore.applySettings(props.schema.componentId, localValues.value)
    message.success('配置已保存')
  } catch (e) {
    message.error('保存失败: ' + String(e))
  } finally {
    saving.value = false
  }
}

async function onReset() {
  dialog.warning({
    title: '确认重置',
    content: '将恢复此组件的所有设置为默认值，确定继续？',
    positiveText: '确定',
    negativeText: '取消',
    onPositiveClick: async () => {
      resetting.value = true
      try {
        await configStore.resetSettings(props.schema.componentId)
        const settings = await configStore.getSettings(props.schema.componentId)
        localValues.value = { ...(settings as Record<string, unknown>) }
        message.success('已重置为默认')
        emit('reload')
      } catch (e) {
        message.error('重置失败: ' + String(e))
      } finally {
        resetting.value = false
      }
    },
  })
}
</script>

<style scoped>
.dynamic-form {
  display: flex;
  flex-direction: column;
  min-height: 0;
  flex: 1 1 auto;
  padding: 16px 24px 0;
}

.form-header {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 4px;
  flex-shrink: 0;
}

.form-header h3 {
  font-size: var(--font-size-lg);
  margin: 0;
}

.form-desc {
  color: var(--text-secondary);
  font-size: var(--font-size-sm);
  margin-bottom: 16px;
  flex-shrink: 0;
}

.form-groups {
  flex: 1;
  overflow-y: auto;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding-bottom: 16px;
}

.form-actions {
  display: flex;
  gap: 8px;
  padding: 12px 0 16px;
  border-top: 1px solid var(--border-color);
  background-color: var(--bg-color);
  flex-shrink: 0;
}
</style>
