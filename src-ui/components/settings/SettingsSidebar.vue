<template>
  <div class="settings-sidebar">
    <n-menu
      :options="menuOptions"
      :value="selectedId"
      :default-expanded-keys="expandedKeys"
      @update:value="onSelect"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
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

const expandedKeys = ref<string[]>(['category_plugins'])

const menuOptions = computed<MenuOption[]>(() => {
  return props.sidebarItems.map(item => {
    const opt: MenuOption = {
      label: item.label,
      key: item.key,
      icon: renderSidebarIcon(item.icon)
    }
    if (item.items && item.items.length > 0) {
      opt.children = item.items.map(subItem => ({
        label: subItem.label,
        key: subItem.key,
        icon: renderSidebarIcon(subItem.icon)
      }))
    }
    return opt
  })
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
