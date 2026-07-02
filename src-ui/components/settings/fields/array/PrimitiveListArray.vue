<template>
  <div class="primitive-list">
    <div v-for="(item, idx) in listValue" :key="idx" class="array-row">
      <n-input
        v-if="itemType === 'text'"
        :value="item as string"
        :disabled="!definition.field.editable"
        size="small"
        @update:value="(val: string) => onUpdate(idx, val)"
      />
      <n-input-number
        v-else-if="itemType === 'number'"
        :value="item as number"
        :min="numConfig.min"
        :max="numConfig.max"
        :step="numConfig.step"
        :disabled="!definition.field.editable"
        size="small"
        @update:value="(val: number | null) => onUpdate(idx, val ?? 0)"
      />
      <n-select
        v-else-if="itemType === 'select'"
        :value="item as string"
        :options="selectOpts"
        :disabled="!definition.field.editable"
        size="small"
        @update:value="(val: string) => onUpdate(idx, val)"
      />
      <div v-else-if="itemType === 'path'" class="path-input-row">
        <n-input
          :value="item as string"
          :disabled="!definition.field.editable"
          size="small"
          @update:value="(val: string) => onUpdate(idx, val)"
        />
        <n-button size="tiny" :disabled="!definition.field.editable" @click="browsePath(idx)">
          浏览
        </n-button>
      </div>
      <n-switch
        v-else-if="itemType === 'boolean'"
        :value="item as boolean"
        :disabled="!definition.field.editable"
        @update:value="(val: boolean) => onUpdate(idx, val)"
      />
      <n-color-picker
        v-else-if="itemType === 'color'"
        :value="item as string"
        :disabled="!definition.field.editable"
        @update:value="(val: string) => onUpdate(idx, val)"
      />
      <n-button
        text type="error" size="tiny"
        :disabled="!definition.field.editable"
        @click="onRemove(idx)"
      >
        删除
      </n-button>
    </div>
    <n-button size="small" :disabled="!definition.field.editable" @click="onAdd">
      添加
    </n-button>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { NInput, NInputNumber, NSelect, NSwitch, NColorPicker, NButton } from 'naive-ui'
import { open } from '@tauri-apps/plugin-dialog'
import {
  getPrimitiveItemType,
  getPrimitiveNumberConfig,
  getPrimitiveSelectOptions,
  getPrimitivePathMode,
  getDefaultArrayItem,
} from '../../../../utils/schemaTypes'
import type { SettingDefinition, ArrayItem } from '../../../../bridge/contract'

const props = defineProps<{
  definition: SettingDefinition
  modelValue: unknown
  item: ArrayItem
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: unknown): void
}>()

const listValue = computed<unknown[]>(() => {
  if (Array.isArray(props.modelValue)) return props.modelValue as unknown[]
  return []
})

const itemType = computed(() => getPrimitiveItemType(props.item))
const numConfig = computed(() => getPrimitiveNumberConfig(props.item))
const selectOpts = computed(() => getPrimitiveSelectOptions(props.item))
const pathMode = computed(() => getPrimitivePathMode(props.item))

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

function onUpdate(idx: number, val: unknown) {
  const arr = [...listValue.value]
  arr[idx] = val
  emit('update:modelValue', arr)
}

async function browsePath(idx: number) {
  try {
    if (pathMode.value === 'directory') {
      const selected = await open({ directory: true, multiple: false })
      if (selected && typeof selected === 'string') onUpdate(idx, selected)
    } else {
      const selected = await open({ multiple: false })
      if (selected && typeof selected === 'string') onUpdate(idx, selected)
    }
  } catch (e) {
    console.error('[PrimitiveListArray] Browse failed:', e)
  }
}
</script>

<style scoped>
.primitive-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.array-row {
  display: flex;
  gap: 6px;
  align-items: center;
}
.array-row > :first-child {
  flex: 1;
}
.path-input-row {
  display: flex;
  gap: 6px;
  align-items: center;
  flex: 1;
}
.path-input-row > :deep(.n-input) {
  flex: 1;
}
</style>
