<template>
  <div class="array-master-detail">
    <div class="md-list">
      <div
        v-for="(_item, idx) in listValue"
        :key="idx"
        class="md-list-item"
        :class="{ active: selectedIndex === idx }"
        @click="selectedIndex = idx"
      >
        <span>{{ summary(idx) }}</span>
        <n-button
          text type="error" size="tiny"
          :disabled="!definition.field.editable"
          @click.stop="onRemove(idx)"
        >
          删除
        </n-button>
      </div>
      <n-button
        size="small"
        :disabled="!definition.field.editable"
        @click="onAdd()"
      >
        添加
      </n-button>
    </div>
    <div class="md-detail">
      <template v-if="selectedIndex < listValue.length">
        <div v-for="fd in fields" :key="fd.key" class="md-field">
          <DynamicFormField
            :definition="{ field: fd, order: 0 }"
            :component-id="componentId"
            :model-value="getField(selectedIndex, fd.key)"
            @update:model-value="(val: unknown) => setField(selectedIndex, fd.key, val)"
          />
        </div>
      </template>
      <n-text v-else depth="3">选择一个项目</n-text>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { NButton, NText } from 'naive-ui'
import DynamicFormField from '../../DynamicFormField.vue'
import { getVisibleObjectFields, getDefaultArrayItem } from '../../../../utils/schemaTypes'
import type { SettingDefinition, ArrayItem } from '../../../../bridge/contract'

const props = defineProps<{
  definition: SettingDefinition
  componentId: string
  modelValue: unknown
  item: ArrayItem
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: unknown): void
}>()

const fields = computed(() => getVisibleObjectFields(props.item))
const selectedIndex = ref(0)

const listValue = computed<unknown[]>(() => {
  if (Array.isArray(props.modelValue)) return props.modelValue as unknown[]
  return []
})

function getField(idx: number, key: string): unknown {
  const item = listValue.value[idx]
  if (item && typeof item === 'object') return (item as Record<string, unknown>)[key]
  return ''
}

function setField(idx: number, key: string, val: unknown) {
  const arr = [...listValue.value]
  const item = arr[idx]
  if (item && typeof item === 'object') {
    arr[idx] = { ...(item as Record<string, unknown>), [key]: val }
  }
  emit('update:modelValue', arr)
}

function summary(idx: number): string {
  const item = listValue.value[idx]
  if (!item || typeof item !== 'object') return `#${idx + 1}`
  if (fields.value.length === 0) return `#${idx + 1}`
  return String((item as Record<string, unknown>)[fields.value[0].key] ?? `#${idx + 1}`)
}

function onAdd() {
  const arr = [...listValue.value]
  arr.push(getDefaultArrayItem(props.item, props.definition.field.defaultValue))
  selectedIndex.value = arr.length - 1
  emit('update:modelValue', arr)
}

function onRemove(idx: number) {
  const arr = [...listValue.value]
  arr.splice(idx, 1)
  if (arr.length === 0) {
    selectedIndex.value = 0
  } else if (idx < selectedIndex.value) {
    selectedIndex.value -= 1
  } else if (idx === selectedIndex.value && selectedIndex.value >= arr.length) {
    selectedIndex.value = arr.length - 1
  }
  emit('update:modelValue', arr)
}
</script>

<style scoped>
.array-master-detail {
  display: flex;
  gap: 8px;
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  overflow: hidden;
}
.md-list {
  width: 160px;
  flex-shrink: 0;
  border-right: 1px solid var(--border-color);
  padding: 6px;
  display: flex;
  flex-direction: column;
  gap: 4px;
  max-height: 300px;
  overflow-y: auto;
}
.md-list-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 4px 6px;
  font-size: var(--font-size-sm);
  border-radius: 4px;
  cursor: pointer;
}
.md-list-item:hover,
.md-list-item.active {
  background: var(--bg-secondary);
}
.md-detail {
  flex: 1;
  padding: 8px;
  overflow-y: auto;
  max-height: 300px;
}
.md-field {
  margin-bottom: 6px;
}
</style>
