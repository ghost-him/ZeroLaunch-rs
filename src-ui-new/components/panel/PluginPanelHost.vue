<template>
  <div class="plugin-panel-host">
    <component
      :is="panelComponent"
      v-if="panelComponent"
      v-bind="panelProps"
    />
    <div v-else class="fallback-panel">
      <n-text depth="3">插件面板: {{ panelType }}</n-text>
      <pre v-if="panelData">{{ JSON.stringify(panelData, null, 2) }}</pre>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { NText } from 'naive-ui'
import { usePluginStore } from '@/stores/plugin-store'
import { useSearchStore } from '@/stores/search-store'

const props = defineProps<{
  panelType: string
  panelData: unknown
}>()

const pluginStore = usePluginStore()
const searchStore = useSearchStore()

const panelComponent = computed(() =>
  pluginStore.getPanelComponent(props.panelType),
)

const panelProps = computed(() => ({
  data: props.panelData,
  actions: searchStore.panelActions,
}))
</script>

<style scoped>
.plugin-panel-host {
  padding: 0;
}

.fallback-panel {
  padding: 16px;
  margin: 8px 16px;
  border-radius: var(--radius-sm);
  background: var(--bg-secondary);
}

pre {
  margin-top: 8px;
  font-size: 11px;
  overflow-x: auto;
  white-space: pre-wrap;
  word-break: break-all;
}
</style>
