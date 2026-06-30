<template>
  <div class="array-table-wrap">
    <table class="array-table">
      <thead>
        <tr>
          <th v-for="fd in fields" :key="fd.key">{{ fd.label }}</th>
          <th v-if="fields.length > 0 && definition.field.editable" class="col-action">操作</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="(_item, idx) in listValue" :key="idx">
          <td v-for="fd in fields" :key="fd.key">
            <n-input
              v-if="isTextType(fd.settingType)"
              :value="getField(idx, fd.key) as string"
              size="small"
              :disabled="!definition.field.editable || !fd.editable"
              @update:value="(val: string) => setField(idx, fd.key, val)"
            />
            <n-input-number
              v-else-if="isFieldNumber(fd)"
              :value="getField(idx, fd.key) as number"
              size="small"
              :min="fieldNumConfig(fd).min"
              :max="fieldNumConfig(fd).max"
              :step="fieldNumConfig(fd).step"
              :disabled="!definition.field.editable || !fd.editable"
              @update:value="(val: number | null) => setField(idx, fd.key, val ?? 0)"
            />
            <n-switch
              v-else-if="isBooleanType(fd.settingType)"
              :value="getField(idx, fd.key) as boolean"
              size="small"
              :disabled="!definition.field.editable || !fd.editable"
              @update:value="(val: boolean) => setField(idx, fd.key, val)"
            />
            <n-select
              v-else-if="isFieldSelect(fd)"
              :value="getField(idx, fd.key) as string"
              :options="fieldSelectOpts(fd)"
              size="small"
              :disabled="!definition.field.editable || !fd.editable"
              @update:value="(val: string) => setField(idx, fd.key, val)"
            />
            <div v-else-if="isFieldPath(fd)" class="table-path-row">
              <n-input
                :value="getField(idx, fd.key) as string"
                size="small"
                :disabled="!definition.field.editable || !fd.editable"
                @update:value="(val: string) => setField(idx, fd.key, val)"
              />
              <n-button
                size="tiny"
                :disabled="!definition.field.editable || !fd.editable"
                @click="browsePath(idx, fd.key, fd)"
              >
                浏览
              </n-button>
            </div>
            <span v-else>{{ getField(idx, fd.key) }}</span>
          </td>
          <td v-if="fields.length > 0 && definition.field.editable" class="col-action">
            <n-button text type="error" size="tiny" @click="onRemove(idx)">
              删除
            </n-button>
          </td>
        </tr>
      </tbody>
    </table>
    <n-button size="small" :disabled="!definition.field.editable" @click="onAdd">
      添加
    </n-button>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { NInput, NInputNumber, NSwitch, NSelect, NButton } from 'naive-ui'
import { open } from '@tauri-apps/plugin-dialog'
import {
  getVisibleObjectFields,
  getDefaultArrayItem,
  isTextType,
  isBooleanType,
  isFieldNumber,
  isFieldSelect,
  isFieldPath,
  getNumberConfig,
  getSelectOptions,
  getPathMode,
} from '../../../../utils/schemaTypes'
import type { SettingDefinition, ArrayItem, FieldDefinition } from '../../../../bridge/contract'

const props = defineProps<{
  definition: SettingDefinition
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

function fieldNumConfig(fd: FieldDefinition) {
  return getNumberConfig(fd.settingType)
}

function fieldSelectOpts(fd: FieldDefinition) {
  return getSelectOptions(fd.settingType)
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

async function browsePath(idx: number, key: string, fd: FieldDefinition) {
  try {
    const mode = getPathMode(fd.settingType)
    const selected = await open({
      directory: mode === 'directory',
      multiple: false,
    })
    if (selected && typeof selected === 'string') setField(idx, key, selected)
  } catch (e) {
    console.error('[ObjectTableArray] Browse path failed:', e)
  }
}
</script>

<style scoped>
.array-table-wrap {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.array-table {
  width: 100%;
  border-collapse: collapse;
  font-size: var(--font-size-sm);
}
.array-table th,
.array-table td {
  padding: 4px 6px;
  border: 1px solid var(--border-color);
  text-align: left;
}
.array-table th {
  background: var(--bg-secondary);
  font-weight: 600;
  white-space: nowrap;
}
.col-action {
  width: 50px;
  text-align: center;
}
.table-path-row {
  display: flex;
  gap: 4px;
  align-items: center;
}
</style>
