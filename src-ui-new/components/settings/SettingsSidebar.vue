<template>
  <div class="settings-sidebar">
    <div
      v-for="group in sidebarItems"
      :key="group.key"
      class="sidebar-group"
    >
      <div class="group-header" @click="toggleGroup(group.key)">
        <n-icon :size="16">
          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle v-if="group.icon === 'settings'" cx="12" cy="12" r="3" />
            <path v-if="group.icon === 'settings'" d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" />
            <path v-if="group.icon === 'search'" d="m21 21-4.3-4.3" />
            <circle v-if="group.icon === 'search'" cx="11" cy="11" r="8" />
            <path v-if="group.icon === 'extension'" d="M20 12V8H6a2 2 0 0 1-2-2c0-2.2 1.8-4 4-4 .7 0 1.4.2 2 .5" />
            <path v-if="group.icon === 'palette'" d="M12 2a10 10 0 0 0 0 20 10 10 0 0 0 0-20z" />
            <path v-if="group.icon === 'palette'" d="M12 2a10 10 0 0 0 0 20" />
            <circle v-if="group.icon === 'info'" cx="12" cy="12" r="10" />
            <line v-if="group.icon === 'info'" x1="12" y1="16" x2="12" y2="12" />
            <line v-if="group.icon === 'info'" x1="12" y1="8" x2="12.01" y2="8" />
          </svg>
        </n-icon>
        <span>{{ group.label }}</span>
        <n-icon v-if="group.type !== 'static' && group.items" :size="12">
          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polyline :points="expandedGroups.has(group.key) ? '6 15 12 9 18 15' : '9 18 15 12 9 6'" />
          </svg>
        </n-icon>
      </div>
      <div v-if="group.type !== 'static' && expandedGroups.has(group.key)" class="group-items">
        <div
          v-for="item in group.items"
          :key="item.componentId"
          class="sidebar-item"
          :class="{ active: selectedId === item.componentId }"
          @click="$emit('select', item.componentId)"
        >
          <div class="item-name">{{ item.componentName }}</div>
          <n-switch
            :value="item.enabled"
            size="small"
            @click.stop
            @update:value="(val: boolean) => $emit('toggle', item.componentId, val)"
          />
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { NIcon, NSwitch } from 'naive-ui'
import type { SidebarItem } from '../../composables/useSettings'

defineProps<{
  sidebarItems: SidebarItem[]
  selectedId: string | null
}>()

defineEmits<{
  (e: 'select', componentId: string): void
  (e: 'toggle', componentId: string, enabled: boolean): void
}>()

const expandedGroups = ref(new Set<string>(['core', 'pipeline', 'plugins']))
function toggleGroup(key: string) {
  if (expandedGroups.value.has(key)) {
    expandedGroups.value.delete(key)
  } else {
    expandedGroups.value.add(key)
  }
}
</script>

<style scoped>
.settings-sidebar {
  width: 220px;
  flex-shrink: 0;
  border-right: 1px solid var(--border-color);
  overflow-y: auto;
  padding: 8px 0;
}

.group-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 16px;
  font-size: var(--font-size-sm);
  font-weight: 600;
  color: var(--text-primary);
  cursor: pointer;
}

.group-header:hover {
  background: var(--bg-secondary);
}

.group-items {
  padding: 0 8px;
}

.sidebar-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 8px 6px 32px;
  border-radius: var(--radius-sm);
  cursor: pointer;
}

.sidebar-item:hover,
.sidebar-item.active {
  background: var(--bg-secondary);
}

.item-name {
  font-size: var(--font-size-sm);
  color: var(--text-primary);
}
</style>
