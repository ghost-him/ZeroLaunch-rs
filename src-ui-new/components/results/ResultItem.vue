<template>
  <component
    :is="customRenderer"
    v-if="customRenderer"
    :item="item"
    :selected="selected"
    :index="index"
  />
  <div
    v-else
    class="result-item"
    :class="{ selected }"
    @click="$emit('click')"
    @dblclick="$emit('dblclick')"
    @contextmenu.prevent="onContextMenu"
  >
    <div class="item-icon">
      <IconDisplay :src="item.icon" :size="24" />
    </div>
    <div class="item-text">
      <div class="item-title">{{ item.title }}</div>
      <div class="item-subtitle" v-if="item.subtitle">{{ item.subtitle }}</div>
    </div>
  </div>
  <ContextMenu
    :visible="ctxVisible"
    :x="ctxX"
    :y="ctxY"
    :items="ctxItems"
    @close="ctxVisible = false"
  />
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import type { Component } from 'vue'
import type { ListItem } from '../../bridge/contract'
import { usePluginStore } from '../../stores/plugin-store'
import IconDisplay from '../common/IconDisplay.vue'
import ContextMenu from '../layout/ContextMenu.vue'

const props = defineProps<{
  item: ListItem
  selected: boolean
  index: number
}>()

const emit = defineEmits<{
  (e: 'click'): void
  (e: 'dblclick'): void
  (e: 'context-action', actionId: string): void
}>()

const pluginStore = usePluginStore()

const customRenderer = computed<Component | null>(() =>
  pluginStore.getResultItemComponent(props.item.targetType),
)

// ---- Context Menu ----
const ctxVisible = ref(false)
const ctxX = ref(0)
const ctxY = ref(0)

const ctxItems = computed(() => {
  return props.item.actions.map((a) => ({
    key: a.id,
    label: a.label,
    action: () => {
      emit('context-action', a.id)
    },
  }))
})

function onContextMenu(e: MouseEvent) {
  ctxX.value = e.clientX
  ctxY.value = e.clientY
  ctxVisible.value = true
}
</script>

<style scoped>
.result-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px 8px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: background-color 0.1s;
}

.result-item:hover,
.result-item.selected {
  background-color: var(--bg-secondary);
}

.item-icon {
  flex-shrink: 0;
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.item-text {
  flex: 1;
  min-width: 0;
}

.item-title {
  font-size: var(--font-size-base);
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.item-subtitle {
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
