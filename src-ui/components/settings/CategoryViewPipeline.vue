<template>
  <div class="category-view-pipeline">
    <PipelineFlowDiagram />

    <!-- 详尽配置 -->
    <n-tabs v-model:value="activeTab" type="line" display-directive="show" class="pipeline-tabs">
      <n-tab-pane v-for="group in groups" :key="group.pane" :name="group.pane" :tab="group.label">
        <ListDetailPanel
          :items="getComponentsByType(group.type)"
          :title="group.label"
          :custom-toggle="!!group.customToggle"
          :toggle-busy="group.customToggle ? searchToggleBusy : false"
          :preselect-id="revealComponentId"
          @toggle="onSearchEngineToggle"
        />
      </n-tab-pane>
    </n-tabs>
  </div>
</template>

<script setup lang="ts">
import { computed, inject, onMounted, ref, watch } from 'vue'
import { NTabs, NTabPane } from 'naive-ui'
import { useI18n } from 'vue-i18n'
import type { ComponentInfo } from '../../bridge/contract'
import { FIELD_REVEAL_KEY, type FieldRevealTarget } from '../../utils/fieldReveal'
import PipelineFlowDiagram from './PipelineFlowDiagram.vue'
import ListDetailPanel from './ListDetailPanel.vue'
import { useSearchEngineToggle } from '../../composables/useSearchEngineToggle'
import { PIPELINE_GROUPS } from '../../utils/pipelineGroups'

const props = defineProps<{
  components: ComponentInfo[]
}>()

const { t } = useI18n()

/** 分组文案随语言切换刷新。 */
const groups = computed(() => PIPELINE_GROUPS.map((g) => ({ ...g, label: t(g.labelKey) })))

const activeTab = ref('datasource')

const reveal = inject(FIELD_REVEAL_KEY, null)
/** 设置助手跳转的目标组件 id（空串表示无）。 */
const revealComponentId = computed(() => reveal?.target.value?.componentId ?? '')

/** 揭示目标属于本分类时切到对应 pane。 */
function focusTargetPane(target: FieldRevealTarget | null | undefined): void {
  if (!target) return
  const component = props.components.find((c) => c.componentId === target.componentId)
  const group = component ? PIPELINE_GROUPS.find((g) => g.type === component.componentType) : undefined
  if (group) activeTab.value = group.pane
}

watch(() => reveal?.target.value, focusTargetPane)
// 从问答页跳转时本组件刚挂载，onMounted 需补一次
onMounted(() => focusTargetPane(reveal?.target.value ?? null))

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
