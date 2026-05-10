<template>
  <div class="category-view-pipeline">
    <PipelineFlowDiagram />

    <!-- 详尽配置 -->
    <n-tabs type="line" default-value="datasource" display-directive="show">
      <n-tab-pane name="datasource" tab="数据源">
        <div class="card-list">
          <CollapsibleComponentCard
            v-for="comp in getComponentsByType('DataSource')"
            :key="comp.componentId"
            :component="comp"
          />
        </div>
      </n-tab-pane>
      <n-tab-pane name="processor" tab="内容处理器">
        <div class="card-list">
          <CollapsibleComponentCard
            v-for="comp in getComponentsByType('KeywordOptimizer')"
            :key="comp.componentId"
            :component="comp"
          />
        </div>
      </n-tab-pane>
      <n-tab-pane name="searchengine" tab="检索引擎">
        <div class="card-list">
          <CollapsibleComponentCard
            v-for="comp in getComponentsByType('SearchEngine')"
            :key="comp.componentId"
            :component="comp"
            custom-toggle
            @toggle="onSearchEngineToggle"
          />
        </div>
      </n-tab-pane>
      <n-tab-pane name="scorebooster" tab="评分增强器">
        <div class="card-list">
          <CollapsibleComponentCard
            v-for="comp in getComponentsByType('ScoreBooster')"
            :key="comp.componentId"
            :component="comp"
          />
        </div>
      </n-tab-pane>
      <n-tab-pane name="executor" tab="执行器">
        <div class="card-list">
          <CollapsibleComponentCard
            v-for="comp in getComponentsByType('ActionExecutor')"
            :key="comp.componentId"
            :component="comp"
          />
        </div>
      </n-tab-pane>
    </n-tabs>
  </div>
</template>

<script setup lang="ts">
import { NTabs, NTabPane } from 'naive-ui'
import type { ComponentInfo } from '../../bridge/contract'
import PipelineFlowDiagram from './PipelineFlowDiagram.vue'
import CollapsibleComponentCard from './CollapsibleComponentCard.vue'
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

.card-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding-top: 16px;
  padding-bottom: 32px;
}
</style>
