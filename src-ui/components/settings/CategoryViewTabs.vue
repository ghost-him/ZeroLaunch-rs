<template>
  <div class="category-view-tabs">
    <!-- 分类下只有一个组件（如外观设置）：没有可切换项，不渲染组件级 tab 条 -->
    <div v-if="isSingle" class="tab-content">
      <ComponentConfigLoader :component="components[0]" :group-tabs="groupTabs" />
    </div>
    <n-tabs v-else type="line" :value="activeTab" display-directive="show" @update:value="onTabChange">
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
import { computed, inject, ref, watch } from 'vue'
import { NTabs, NTabPane } from 'naive-ui'
import { resolveText } from '../../i18n'
import type { ComponentInfo } from '../../bridge/contract'
import ComponentConfigLoader from './ComponentConfigLoader.vue'
import { FIELD_REVEAL_KEY, type FieldRevealTarget } from '../../utils/fieldReveal'
import { UNSAVED_CHANGES_KEY } from '../../utils/unsavedChanges'

const props = withDefaults(defineProps<{
  components: ComponentInfo[]
  /** 组件内按分组渲染为 tab 页（透传给 ComponentConfigLoader） */
  groupTabs?: boolean
}>(), {
  groupTabs: false,
})

const activeTab = ref<string | undefined>(props.components[0]?.componentId)
/** 分类下只有一个组件时无组件可切换，组件级 tab 条连同其选中态一并省略。 */
const isSingle = computed(() => props.components.length === 1)
const unsavedChanges = inject(UNSAVED_CHANGES_KEY, null)

/** 切换组件页签：存在未保存修改时先确认丢弃，取消则留在当前页签。 */
async function onTabChange(value: string | number): Promise<void> {
  const next = String(value)
  if (next === activeTab.value) return
  if (unsavedChanges && !await unsavedChanges.confirmDiscard()) return
  activeTab.value = next
}

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

// immediate：跳转请求可能早于本组件挂载，首次渲染即落到目标页签
watch(revealTarget, focusTargetTab, { immediate: true })
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
