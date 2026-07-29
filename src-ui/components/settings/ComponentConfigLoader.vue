<template>
  <div class="component-config-loader" :class="{ 'is-loaded': schema && settings && !loading && !loadErr }">
    <div v-if="loading" class="loading-state">
      <n-spin :size="20" />
    </div>
    <div v-else-if="loadErr" class="error-state">
      <n-text type="error">{{ loadErr }}</n-text>
      <n-button size="small" @click="init">{{ $t('settings.saveFailed') }}</n-button>
    </div>
    <component
      v-else-if="settingsComponent && settings"
      :is="settingsComponent"
      :current-settings="settings"
      @save="onThirdPartySave"
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
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { NSpin, NText, NButton } from 'naive-ui'
import DynamicForm from './DynamicForm.vue'
import { useConfigStore } from '../../stores/config-store'
import { usePluginStore } from '../../stores/plugin-store'
import { onConfigChanged } from '../../bridge/events'
import { configApplySettings } from '../../bridge/commands'
import type { ComponentInfo, ComponentSchema } from '../../bridge/contract'

const props = defineProps<{
  component: ComponentInfo
}>()

const configStore = useConfigStore()
const pluginStore = usePluginStore()

const loading = ref(true)
const loadErr = ref<string | null>(null)
const schema = ref<ComponentSchema | null>(null)
const settings = ref<Record<string, unknown> | null>(null)

// 第三方插件自定义设置页组件
const settingsComponent = computed(() =>
  pluginStore.getSettingsComponent(props.component.componentId),
)

let unlistenConfig: (() => void) | null = null

async function onThirdPartySave(newSettings: unknown) {
  try {
    await configApplySettings(props.component.componentId, newSettings)
    await init()
  } catch (e) {
    loadErr.value = String(e)
  }
}

async function init() {
  loading.value = true
  loadErr.value = null
  try {
    const [s, cfg] = await Promise.all([
      configStore.getSchema(props.component.componentId),
      configStore.getSettings(props.component.componentId),
    ])
    schema.value = s
    settings.value = cfg as Record<string, unknown>
  } catch (e) {
    loadErr.value = String(e)
  } finally {
    loading.value = false
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
