<template>
  <div class="category-view-pipeline">
    <PipelineFlowDiagram />

    <!-- 详尽配置 -->
    <n-tabs type="line" default-value="datasource" display-directive="show" class="pipeline-tabs">
      <n-tab-pane name="datasource" :tab="t('settings.dataSource')">
        <ListDetailPanel
          :items="getComponentsByType('DataSource')"
          :title="t('settings.dataSource')"
        />
      </n-tab-pane>
      <n-tab-pane name="processor" :tab="t('settings.contentProcessor')">
        <ListDetailPanel
          :items="getComponentsByType('KeywordOptimizer')"
          :title="t('settings.contentProcessor')"
        />
      </n-tab-pane>
      <n-tab-pane name="injector" :tab="t('settings.keywordInjector')">
        <ListDetailPanel
          :items="getComponentsByType('KeywordInjector')"
          :title="t('settings.keywordInjector')"
        />
      </n-tab-pane>
      <n-tab-pane name="bias" :tab="t('settings.biasRule')">
        <ListDetailPanel
          :items="getComponentsByType('BiasRule')"
          :title="t('settings.biasRule')"
        />
      </n-tab-pane>
      <n-tab-pane name="exclusion" :tab="t('settings.exclusionRule')">
        <ListDetailPanel
          :items="getComponentsByType('ExclusionRule')"
          :title="t('settings.exclusionRule')"
        />
      </n-tab-pane>
      <n-tab-pane name="searchengine" :tab="t('settings.searchEngine')">
        <ListDetailPanel
          :items="getComponentsByType('SearchEngine')"
          :title="t('settings.searchEngine')"
          custom-toggle
          :toggle-busy="searchToggleBusy"
          @toggle="onSearchEngineToggle"
        />
      </n-tab-pane>
      <n-tab-pane name="scorebooster" :tab="t('settings.scoreBooster')">
        <ListDetailPanel
          :items="getComponentsByType('ScoreBooster')"
          :title="t('settings.scoreBooster')"
        />
      </n-tab-pane>
      <n-tab-pane name="executor" :tab="t('settings.actionExecutor')">
        <ListDetailPanel
          :items="getComponentsByType('ActionExecutor')"
          :title="t('settings.actionExecutor')"
        />
      </n-tab-pane>
    </n-tabs>
  </div>
</template>

<script setup lang="ts">
import { NTabs, NTabPane } from 'naive-ui'
import { useI18n } from 'vue-i18n'
import type { ComponentInfo } from '../../bridge/contract'
import PipelineFlowDiagram from './PipelineFlowDiagram.vue'
import ListDetailPanel from './ListDetailPanel.vue'
import { useSearchEngineToggle } from '../../composables/useSearchEngineToggle'

const props = defineProps<{
  components: ComponentInfo[]
}>()

const { t } = useI18n()

function getComponentsByTypes(types: string[]): ComponentInfo[] {
  return props.components.filter(c => types.includes(c.componentType))
}

function getComponentsByType(type: string): ComponentInfo[] {
  return getComponentsByTypes([type])
}

const { onToggle: onSearchEngineToggle, toggling: searchToggleBusy } = useSearchEngineToggle(
  () => getComponentsByType('SearchEngine'),
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
