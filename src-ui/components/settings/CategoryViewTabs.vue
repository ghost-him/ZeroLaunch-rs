<template>
  <div class="category-view-tabs">
    <n-tabs type="line" v-model:value="activeTab" display-directive="show">
      <n-tab-pane
        v-for="comp in components"
        :key="comp.componentId"
        :name="comp.componentId"
        :tab="comp.componentName"
      >
        <div class="tab-content">
          <ComponentConfigLoader :component="comp" />
        </div>
      </n-tab-pane>
    </n-tabs>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { NTabs, NTabPane } from 'naive-ui'
import type { ComponentInfo } from '../../bridge/contract'
import ComponentConfigLoader from './ComponentConfigLoader.vue'

const props = defineProps<{
  components: ComponentInfo[]
}>()

const activeTab = ref<string | undefined>(props.components[0]?.componentId)

watch(() => props.components, (newVal) => {
  if (newVal.length > 0 && !newVal.find(c => c.componentId === activeTab.value)) {
    activeTab.value = newVal[0].componentId
  }
}, { immediate: true })
</script>

<style scoped>
.category-view-tabs {
  height: 100%;
}
.tab-content {
  padding-top: 16px;
  padding-bottom: 32px;
}
</style>
