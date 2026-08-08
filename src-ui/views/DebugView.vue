<template>
  <div class="debug-view">
    <n-tabs type="line" v-model:value="activeTab" display-directive="show">
      <n-tab-pane name="inspector" :tab="t('inspector.title')">
        <PluginInspector />
      </n-tab-pane>
      <n-tab-pane name="tools" :tab="t('debug.title')">
        <DebugTools />
      </n-tab-pane>
    </n-tabs>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { NTabs, NTabPane } from 'naive-ui'
import PluginInspector from '../components/debug/PluginInspector.vue'
import DebugTools from '../components/debug/DebugTools.vue'

const { t } = useI18n()
const activeTab = ref('inspector')
</script>

<style scoped>
.debug-view {
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  padding: 16px 24px 0;
}
.debug-view :deep(.n-tabs) {
  display: flex;
  flex-direction: column;
  height: 100%;
}
.debug-view :deep(.n-tabs-pane-wrapper) {
  flex: 1;
  overflow: auto;
}
.debug-view :deep(.n-tab-pane) {
  height: 100%;
  padding-top: 16px;
}
</style>
