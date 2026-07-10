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

function getComponentsByType(type: string): ComponentInfo[] {
  return props.components.filter(c => c.componentType === type)
}

const { onToggle: onSearchEngineToggle } = useSearchEngineToggle(
  () => getComponentsByType('SearchEngine')
)
</script>

<style scoped>
.category-view-pipeline {
  height: 100%;
}

.pipeline-tabs {
  height: calc(100% - 100px);
}

.pipeline-tabs :deep(.n-tabs-nav--line) {
  margin-bottom: 0;
}

.pipeline-tabs :deep(.n-tab-pane) {
  padding-top: 16px;
  height: calc(100% - 44px);
}
</style>
