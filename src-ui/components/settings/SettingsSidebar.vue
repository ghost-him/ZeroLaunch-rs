<template>
  <div class="settings-sidebar">
    <n-menu
      :options="menuOptions"
      :value="selectedId"
      @update:value="onSelect"
    />
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { NMenu } from 'naive-ui'
import type { MenuOption } from 'naive-ui'
import type { SidebarCategory } from '../../utils/settingsSidebar'
import { renderSidebarIcon } from './sidebarIcons'

const props = defineProps<{
  sidebarItems: SidebarCategory[]
  selectedId: string | null
}>()

const emit = defineEmits<{
  (e: 'select', key: string): void
}>()

const menuOptions = computed<MenuOption[]>(() => {
  return props.sidebarItems.map(item => ({
    label: item.label,
    key: item.key,
    icon: renderSidebarIcon(item.icon)
  }))
})

function onSelect(key: string) {
  emit('select', key)
}
</script>

<style scoped>
.settings-sidebar {
  width: 200px;
  flex-shrink: 0;
  border-right: 1px solid var(--border-color);
  overflow-y: auto;
  padding: 8px 0;
}
</style>
