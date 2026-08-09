<template>
  <div class="category-view-list">
    <div
      v-for="comp in components"
      :key="comp.componentId"
      class="config-section"
    >
      <div v-if="components.length > 1" class="section-title">
        {{ resolveText(comp.componentName) }}
      </div>
      <ComponentConfigLoader :component="comp" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { resolveText } from '../../i18n'
import type { ComponentInfo } from '../../bridge/contract'
import ComponentConfigLoader from './ComponentConfigLoader.vue'

defineProps<{
  components: ComponentInfo[]
}>()
</script>

<style scoped>
.category-view-list {
  display: flex;
  flex-direction: column;
  gap: 24px;
  padding-bottom: 24px;
}
.config-section {
  background: var(--bg-color-secondary);
  padding: 0;
  border-radius: 8px;
  border: 1px solid var(--border-color);
  overflow: hidden;
}
.section-title {
  font-size: 16px;
  font-weight: 600;
  margin-bottom: 0;
  color: var(--text-color);
  padding: 16px 20px;
  border-bottom: 1px solid var(--border-color);
}
</style>
