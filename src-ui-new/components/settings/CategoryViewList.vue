<template>
  <div class="category-view-list">
    <template v-if="showToggle">
      <CollapsibleComponentCard
        v-for="comp in components"
        :key="comp.componentId"
        :component="comp"
      />
    </template>
    <template v-else>
      <div
        v-for="comp in components"
        :key="comp.componentId"
        class="config-section"
      >
        <div v-if="components.length > 1" class="section-title">
          {{ comp.componentName }}
        </div>
        <ComponentConfigLoader :component="comp" />
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import type { ComponentInfo } from '../../bridge/contract'
import ComponentConfigLoader from './ComponentConfigLoader.vue'
import CollapsibleComponentCard from './CollapsibleComponentCard.vue'

defineProps<{
  components: ComponentInfo[]
  showToggle?: boolean
}>()
</script>

<style scoped>
.category-view-list {
  display: flex;
  flex-direction: column;
  gap: 24px;
}
.config-section {
  background: var(--bg-color-secondary);
  padding: 20px;
  border-radius: 8px;
  border: 1px solid var(--border-color);
}
.section-title {
  font-size: 16px;
  font-weight: 600;
  margin-bottom: 16px;
  color: var(--text-color);
  padding-bottom: 8px;
  border-bottom: 1px solid var(--border-color);
}
</style>
