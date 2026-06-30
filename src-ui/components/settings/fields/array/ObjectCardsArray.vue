<template>
  <div class="object-cards">
    <div v-for="(_item, idx) in listValue" :key="idx" class="object-card">
      <div class="object-card-header">
        <span class="card-index">#{{ idx + 1 }}</span>
        <n-button
          text type="error" size="tiny"
          :disabled="!definition.field.editable"
          @click="onRemove(idx)"
        >
          删除
        </n-button>
      </div>
      <div v-for="fd in fields" :key="fd.key" class="object-card-field">
        <DynamicFormField
          :definition="{ field: fd, order: 0 }"
          :component-id="componentId"
          :model-value="getField(idx, fd.key)"
          @update:model-value="(val: unknown) => setField(idx, fd.key, val)"
        />
      </div>
    </div>
    <n-button size="small" :disabled="!definition.field.editable" @click="onAdd">
      添加
    </n-button>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { NButton } from 'naive-ui'
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

function onAdd() {
  const arr = [...listValue.value]
  arr.push(getDefaultArrayItem(props.item, props.definition.field.defaultValue))
  emit('update:modelValue', arr)
}

function onRemove(idx: number) {
  const arr = [...listValue.value]
  arr.splice(idx, 1)
  emit('update:modelValue', arr)
}
</script>

<style scoped>
.object-cards {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.object-card {
  border: 1px solid var(--border-color);
  border-radius: var(--radius-sm);
  padding: 8px 10px;
}
.object-card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 6px;
}
.card-index {
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
  font-weight: 500;
}
.object-card-field {
  margin-bottom: 6px;
}
</style>
