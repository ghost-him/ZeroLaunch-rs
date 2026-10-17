<template>
  <div class="category-view-tabs">
    <n-tabs type="line" v-model:value="activeTab" display-directive="show">
      <n-tab-pane
        v-for="comp in components"
        :key="comp.componentId"
        :name="comp.componentId"
        :tab="resolveText(comp.componentName)"
      >
        <div class="tab-content">
          <ComponentConfigLoader :component="comp" :group-tabs="groupTabs" />
        </div>
      </n-tab-pane>
    </n-tabs>
  </div>
</template>

<script setup lang="ts">
import { inject, onMounted, ref, watch } from 'vue'
import { NTabs, NTabPane } from 'naive-ui'
import { resolveText } from '../../i18n'
import type { ComponentInfo } from '../../bridge/contract'
import ComponentConfigLoader from './ComponentConfigLoader.vue'
import { FIELD_REVEAL_KEY, type FieldRevealTarget } from '../../utils/fieldReveal'

const props = withDefaults(defineProps<{
  components: ComponentInfo[]
  /** 组件内按分组渲染为 tab 页（透传给 ComponentConfigLoader） */
  groupTabs?: boolean
}>(), {
  groupTabs: false,
})

const activeTab = ref<string | undefined>(props.components[0]?.componentId)

watch(() => props.components, (newVal) => {
  if (newVal.length > 0 && !newVal.find(c => c.componentId === activeTab.value)) {
    activeTab.value = newVal[0].componentId
  }
}, { immediate: true })

// 设置助手跳转：目标组件在本分类时先切到它的 tab，字段滚动由 DynamicForm 负责
const reveal = inject(FIELD_REVEAL_KEY, null)

/** 当前待揭示位置（无提供方时为 null）。 */
function revealTarget(): FieldRevealTarget | null {
  return reveal?.target.value ?? null
}

/** 目标组件在本分类内时切到它的 tab。 */
function focusTargetTab(target: FieldRevealTarget | null): void {
  if (!target) return
  if (props.components.some((component) => component.componentId === target.componentId)) {
    activeTab.value = target.componentId
  }
}

watch(revealTarget, focusTargetTab)
// 从问答页跳转时本组件是刚挂载的：watch 看不到已发生的变更，需在挂载时补一次
onMounted(() => focusTargetTab(revealTarget()))
</script>

<style scoped>
.category-view-tabs {
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.category-view-tabs :deep(.n-tabs) {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.category-view-tabs :deep(.n-tabs-pane-wrapper) {
  flex: 1;
  overflow: hidden;
}

.category-view-tabs :deep(.n-tab-pane) {
  height: 100%;
  overflow: hidden;
}

.tab-content {
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  padding-top: 16px;
}
</style>
