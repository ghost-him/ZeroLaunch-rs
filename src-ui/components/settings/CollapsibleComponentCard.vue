<template>
  <div class="collapsible-card" :class="{ 'is-expanded': isExpanded }">
    <div class="card-header" @click="toggleExpand">
      <div class="header-left">
        <div class="icon-box">
          <n-icon :size="18">
            <DollarSign />
          </n-icon>
        </div>
        <div class="title-details">
          <div class="card-title">{{ component.componentName }}</div>
          <div class="card-subtitle">{{ component.componentId }}</div>
        </div>
      </div>
      <div class="header-right">
        <n-switch
          :value="component.enabled"
          size="medium"
          @click.stop
          @update:value="onToggle"
        />
        <div class="expand-icon">
          <n-icon :size="16">
            <ChevronDown v-if="isExpanded" />
            <ChevronUp v-else />
          </n-icon>
        </div>
      </div>
    </div>
    
    <div v-show="isExpanded" class="card-body">
      <ComponentConfigLoader :component="component" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { NIcon, NSwitch } from 'naive-ui'
import type { ComponentInfo } from '../../bridge/contract'
import ComponentConfigLoader from './ComponentConfigLoader.vue'
import { useConfigStore } from '../../stores/config-store'
import { ChevronDown, ChevronUp, DollarSign } from 'lucide-vue-next'

const props = defineProps<{
  component: ComponentInfo
  customToggle?: boolean
}>()

const emit = defineEmits<{
  (e: 'toggle', componentId: string, enabled: boolean): void
}>()

const isExpanded = ref(false)
const configStore = useConfigStore()

function toggleExpand() {
  isExpanded.value = !isExpanded.value
}

async function onToggle(val: boolean) {
  if (props.customToggle) {
    emit('toggle', props.component.componentId, val)
    return
  }
  try {
    await configStore.setEnabled(props.component.componentId, val)
  } catch (e) {
    console.error('Failed to toggle component:', e)
  }
}
</script>

<style scoped>
.collapsible-card {
  border: 1px solid var(--border-color);
  border-radius: 8px;
  margin-bottom: 12px;
  background-color: var(--bg-color-secondary);
  transition: border-color 0.2s, box-shadow 0.2s;
  overflow: hidden;
}

.collapsible-card.is-expanded {
  border-color: var(--primary-color-alpha);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.05);
}

.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  cursor: pointer;
  user-select: none;
  background-color: transparent;
  transition: background-color 0.2s;
}

.card-header:hover {
  background-color: var(--hover-color);
}

.header-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.icon-box {
  width: 32px;
  height: 32px;
  border-radius: 6px;
  background-color: var(--primary-color-alpha);
  color: var(--primary-color);
  display: flex;
  align-items: center;
  justify-content: center;
}

.title-details {
  display: flex;
  flex-direction: column;
}

.card-title {
  font-size: 14px;
  font-weight: 500;
  color: var(--text-color);
}

.card-subtitle {
  font-size: 12px;
  color: var(--text-color-secondary);
  margin-top: 2px;
}

.header-right {
  display: flex;
  align-items: center;
  gap: 16px;
}

.expand-icon {
  color: var(--text-color-secondary);
  display: flex;
  align-items: center;
  justify-content: center;
  transition: transform 0.3s;
}

.card-body {
  padding: 0 0 16px;
  border-top: 1px solid var(--border-color);
  background-color: var(--bg-color);
}
</style>
