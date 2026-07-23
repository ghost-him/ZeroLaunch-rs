<template>
  <div
    class="component-config-loader"
    :class="{ 'is-loaded': ready }"
  >
    <div v-if="loading" class="loading-state">
      <n-spin :size="20" />
    </div>
    <div v-else-if="loadErr" class="error-state">
      <n-text type="error">{{ loadErr }}</n-text>
      <n-button size="small" @click="init">{{ $t('settings.saveFailed') }}</n-button>
    </div>
    <component
      :is="customSettings"
      v-else-if="customSettings && settings"
      :key="component.componentId + '-custom'"
      :current-settings="settings"
      @save="onCustomSave"
    />
    <DynamicForm
      v-else-if="schema && settings"
      :key="component.componentId"
      :schema="schema"
      :current-settings="settings"
      @reload="init"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted } from 'vue'
import { NSpin, NText, NButton, useMessage } from 'naive-ui'
import DynamicForm from './DynamicForm.vue'
import { useConfigStore } from '../../stores/config-store'
import { usePluginStore } from '../../stores/plugin-store'
import { onConfigChanged } from '../../bridge/events'
import type { ComponentInfo, ComponentSchema } from '../../bridge/contract'

const props = defineProps<{
  component: ComponentInfo
}>()

const configStore = useConfigStore()
const pluginStore = usePluginStore()
const message = useMessage()

const loading = ref(true)
const loadErr = ref<string | null>(null)
const schema = ref<ComponentSchema | null>(null)
const settings = ref<Record<string, unknown> | null>(null)

const customSettings = computed(() =>
  pluginStore.getSettingsComponent(props.component.componentId),
)

const ready = computed(
  () =>
    !loading.value &&
    !loadErr.value &&
    !!settings.value &&
    (!!customSettings.value || !!schema.value),
)

let unlistenConfig: (() => void) | null = null

async function init() {
  loading.value = true
  loadErr.value = null
  try {
    // 有自定义设置页时仍拉 settings；schema 仅 DynamicForm 需要
    if (customSettings.value) {
      const cfg = await configStore.getSettings(props.component.componentId)
      settings.value = cfg as Record<string, unknown>
      schema.value = null
    } else {
      const [s, cfg] = await Promise.all([
        configStore.getSchema(props.component.componentId),
        configStore.getSettings(props.component.componentId),
      ])
      schema.value = s
      settings.value = cfg as Record<string, unknown>
    }
  } catch (e) {
    loadErr.value = String(e)
  } finally {
    loading.value = false
  }
}

async function onCustomSave(next: unknown) {
  try {
    await configStore.applySettings(props.component.componentId, next)
    const cfg = await configStore.getSettings(props.component.componentId)
    settings.value = cfg as Record<string, unknown>
    message.success('配置已保存')
  } catch (e) {
    message.error('保存失败: ' + String(e))
  }
}

onMounted(async () => {
  await init()
  unlistenConfig = await onConfigChanged((payload) => {
    if (payload.componentId === props.component.componentId) {
      init()
    }
  })
})

onUnmounted(() => {
  unlistenConfig?.()
})
</script>

<style scoped>
.component-config-loader {
  display: flex;
  flex-direction: column;
  padding: 8px 0;
}

.component-config-loader.is-loaded {
  flex: 1;
  min-height: 0;
  padding: 0;
}

.loading-state,
.error-state {
  padding: 16px;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
}
</style>
