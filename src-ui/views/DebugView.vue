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
/* 弹性填充链：settings-content(flex column) → debug-view(flex:1) → n-tabs(flex:1) → n-tab-pane(flex:1 + overflow) */
.debug-view {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  padding: 16px 24px 0;
}
.debug-view :deep(.n-tabs) {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
}
.debug-view :deep(.n-tabs-nav) {
  flex-shrink: 0;
}
/* naive-ui 非 animated 分支无 .n-tabs-pane-wrapper 包装层，pane 是 n-tabs 直接子元素，
   滚动由激活的 pane 接管（display-directive=show 时非激活 pane 为 display:none） */
.debug-view :deep(.n-tab-pane) {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding-top: 16px;
}
</style>
