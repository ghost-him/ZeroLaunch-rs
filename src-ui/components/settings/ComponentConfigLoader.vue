<template>
  <div class="component-config-loader">
    <div v-if="loading" class="loading-state">
      <n-spin :size="20" />
    </div>
    <div v-else-if="loadErr" class="error-state">
      <n-text type="error">{{ loadErr }}</n-text>
      <n-button size="small" @click="init">{{ $t('settings.saveFailed') }}</n-button>
    </div>
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
import { ref, onMounted, onUnmounted } from 'vue'
import { NSpin, NText, NButton } from 'naive-ui'
import DynamicForm from './DynamicForm.vue'
import { useConfigStore } from '../../stores/config-store'
import { onConfigChanged } from '../../bridge/events'
import type { ComponentInfo, ComponentSchema } from '../../bridge/contract'

const props = defineProps<{
  component: ComponentInfo
}>()

const configStore = useConfigStore()

const loading = ref(true)
const loadErr = ref<string | null>(null)
const schema = ref<ComponentSchema | null>(null)
const settings = ref<Record<string, unknown> | null>(null)

let unlistenConfig: (() => void) | null = null

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
  padding: 8px 0;
}
.loading-state, .error-state {
  padding: 16px;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
}
</style>
