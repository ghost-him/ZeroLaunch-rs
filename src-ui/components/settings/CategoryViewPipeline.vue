<template>
  <div class="category-view-pipeline">
    <PipelineFlowDiagram />

    <!-- 详尽配置 -->
    <n-tabs type="line" default-value="datasource" display-directive="show" class="pipeline-tabs">
      <n-tab-pane name="datasource" tab="数据源">
        <ListDetailPanel
          :items="getComponentsByType('DataSource')"
          title="数据源"
        />
      </n-tab-pane>
      <n-tab-pane name="processor" tab="内容处理器">
        <ListDetailPanel
          :items="getComponentsByType('KeywordOptimizer')"
          title="内容处理器"
        />
      </n-tab-pane>
      <n-tab-pane name="injector" tab="关键字注入器">
        <ListDetailPanel
          :items="getComponentsByType('KeywordInjector')"
          title="关键字注入器"
        />
      </n-tab-pane>
      <n-tab-pane name="searchengine" tab="检索引擎">
        <ListDetailPanel
          :items="getComponentsByType('SearchEngine')"
          title="检索引擎"
          custom-toggle
          @toggle="onSearchEngineToggle"
        />
      </n-tab-pane>
      <n-tab-pane name="scorebooster" tab="评分增强器">
        <ListDetailPanel
          :items="getComponentsByType('ScoreBooster')"
          title="评分增强器"
        />
      </n-tab-pane>
      <n-tab-pane name="executor" tab="执行器">
        <ListDetailPanel
          :items="getComponentsByType('ActionExecutor')"
          title="执行器"
        />
      </n-tab-pane>
    </n-tabs>
  </div>
</template>

<script setup lang="ts">
import { NTabs, NTabPane } from 'naive-ui'
import type { ComponentInfo } from '../../bridge/contract'
import PipelineFlowDiagram from './PipelineFlowDiagram.vue'
import ListDetailPanel from './ListDetailPanel.vue'
import { useSearchEngineToggle } from '../../composables/useSearchEngineToggle'

const props = defineProps<{
  components: ComponentInfo[]
}>()

function getComponentsByTypes(types: string[]): ComponentInfo[] {
  return props.components.filter(c => types.includes(c.componentType))
}

function getComponentsByType(type: string): ComponentInfo[] {
  return getComponentsByTypes([type])
}

const { onToggle: onSearchEngineToggle } = useSearchEngineToggle(
  () => getComponentsByType('SearchEngine')
)
</script>

<style scoped>
.category-view-pipeline {
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.pipeline-tabs {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  padding-bottom: 16px;
}

/* n-tabs 根元素同时带有 .n-tabs 类，需要更高优先级覆盖其 display: block */
.pipeline-tabs.n-tabs {
  display: flex;
  flex-direction: column;
}

.pipeline-tabs :deep(.n-tabs-nav--line) {
  margin-bottom: 0;
  flex-shrink: 0;
}

.pipeline-tabs :deep(.n-tabs-pane-wrapper) {
  flex: 1;
  overflow: hidden;
}

.pipeline-tabs :deep(.n-tab-pane) {
  height: 100%;
  overflow: hidden;
  padding-top: 16px;
}
</style>
