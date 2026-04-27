<template>
  <div class="plugin-panel-host">
    <!-- TODO: 后续从 PluginManager 获取注册的面板组件，此处为降级显示 -->
    <div class="fallback-panel">
      <n-text depth="3">插件面板: {{ panelType }}</n-text>
      <pre v-if="panelData">{{ JSON.stringify(panelData, null, 2) }}</pre>
    </div>
  </div>
</template>

<script setup lang="ts">
import { NText } from 'naive-ui'

defineProps<{
  panelType: string
  panelData: unknown
}>()
</script>

<style scoped>
.plugin-panel-host {
  padding: 16px;
}

.fallback-panel {
  padding: 16px;
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
