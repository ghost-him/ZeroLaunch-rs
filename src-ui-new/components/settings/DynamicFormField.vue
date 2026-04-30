<template>
  <div class="form-field" v-if="definition.field.visible">
    <label class="field-label">
      {{ definition.field.label }}
      <span class="field-desc" v-if="definition.field.description"> — {{ definition.field.description }}</span>
    </label>

    <div class="field-control">
      <!-- Inline types: input + optional config action button -->
      <div v-if="isInlineType" class="field-input-row">
        <n-input
          v-if="isSimple('Text')"
          :value="modelValue as string"
          :disabled="!definition.field.editable"
          :placeholder="definition.field.description"
          @update:value="$emit('update:modelValue', $event)"
        />

        <n-input-number
          v-else-if="isNumber"
          :value="modelValue as number"
          :min="numberConfig.min"
          :max="numberConfig.max"
          :step="numberConfig.step"
          :disabled="!definition.field.editable"
          @update:value="(val: number | null) => $emit('update:modelValue', val ?? 0)"
        />

        <div v-else-if="isPath" class="path-input-row">
          <n-input
            :value="modelValue as string"
            :disabled="!definition.field.editable"
            :placeholder="pathPlaceholder"
            @update:value="$emit('update:modelValue', $event)"
          />
          <n-button
            size="small"
            :disabled="!definition.field.editable"
            @click="browsePath"
          >
            浏览
          </n-button>
        </div>

        <n-select
          v-else-if="isSelect"
          :value="modelValue as string"
          :options="selectOptions"
          :disabled="!definition.field.editable"
          @update:value="$emit('update:modelValue', $event)"
        />

        <n-color-picker
          v-else-if="isSimple('Color')"
          :value="modelValue as string"
          :disabled="!definition.field.editable"
          @update:value="$emit('update:modelValue', $event)"
        />

        <ConfigActionButton
          v-if="definition.configAction"
          :component-id="componentId"
          :config-action="definition.configAction"
          :field-key="definition.field.key"
          :editable="definition.field.editable"
          :model-value="modelValue"
          @update:model-value="$emit('update:modelValue', $event)"
        />
      </div>

      <!-- Boolean: switch -->
      <div v-else-if="isSimple('Boolean')" class="field-input-row">
        <n-switch
          :value="modelValue as boolean"
          :disabled="!definition.field.editable"
          @update:value="$emit('update:modelValue', $event)"
        />
        <ConfigActionButton
          v-if="definition.configAction"
          :component-id="componentId"
          :config-action="definition.configAction"
          :field-key="definition.field.key"
          :editable="definition.field.editable"
          :model-value="modelValue"
          @update:model-value="$emit('update:modelValue', $event)"
        />
      </div>

      <!-- Json: textarea -->
      <div v-else-if="isSimple('Json')">
        <n-input
          type="textarea"
          :value="jsonString"
          :disabled="!definition.field.editable"
          :autosize="{ minRows: 2, maxRows: 6 }"
          @update:value="onJsonInput"
        />
      </div>

      <!-- Array types -->
      <div v-else-if="isArray" class="array-field">
        <!-- Primitive tags mode -->
        <div v-if="isPrimitiveArray && arrayUiHint === 'Tags'" class="array-tags">
          <n-dynamic-tags
            :value="arrayValue as string[]"
            :disabled="!definition.field.editable"
            @update:value="onArrayUpdate"
          />
        </div>

        <!-- Primitive default mode: simple list -->
        <div v-else-if="isPrimitiveArray" class="array-default">
          <div
            v-for="(item, idx) in arrayValue"
            :key="idx"
            class="array-row"
          >
            <n-input
              v-if="primitiveItemType === 'Text'"
              :value="item as string"
              :disabled="!definition.field.editable"
              size="small"
              @update:value="(val: string) => onArrayItemUpdate(idx, val)"
            />
            <n-input-number
              v-else-if="primitiveItemType === 'Number'"
              :value="item as number"
              :min="numberPrimitiveConfig.min"
              :max="numberPrimitiveConfig.max"
              :step="numberPrimitiveConfig.step"
              :disabled="!definition.field.editable"
              size="small"
              @update:value="(val: number | null) => onArrayItemUpdate(idx, val ?? 0)"
            />
            <n-select
              v-else-if="primitiveItemType === 'Select'"
              :value="item as string"
              :options="primitiveSelectOptions"
              :disabled="!definition.field.editable"
              size="small"
              @update:value="(val: string) => onArrayItemUpdate(idx, val)"
            />
            <div v-else-if="primitiveItemType === 'Path'" class="path-input-row array-path">
              <n-input
                :value="item as string"
                :disabled="!definition.field.editable"
                size="small"
                @update:value="(val: string) => onArrayItemUpdate(idx, val)"
              />
              <n-button
                size="tiny"
                :disabled="!definition.field.editable"
                @click="browseArrayPath(idx)"
              >
                浏览
              </n-button>
            </div>
            <n-switch
              v-else-if="primitiveItemType === 'Boolean'"
              :value="item as boolean"
              :disabled="!definition.field.editable"
              @update:value="(val: boolean) => onArrayItemUpdate(idx, val)"
            />
            <n-color-picker
              v-else-if="primitiveItemType === 'Color'"
              :value="item as string"
              :disabled="!definition.field.editable"
              @update:value="(val: string) => onArrayItemUpdate(idx, val)"
            />
            <n-button
              text
              type="error"
              size="tiny"
              :disabled="!definition.field.editable"
              @click="removeArrayItem(idx)"
            >
              删除
            </n-button>
          </div>
          <n-button
            size="small"
            :disabled="!definition.field.editable"
            @click="addArrayItem"
          >
            添加
          </n-button>
        </div>

        <!-- Object array: MasterDetail mode -->
        <div v-else-if="isObjectArray && arrayUiHint === 'MasterDetail'" class="array-master-detail">
          <div class="md-list">
            <div
              v-for="(_item, idx) in arrayValue"
              :key="idx"
              class="md-list-item"
              :class="{ active: masterSelectedIndex === idx }"
              @click="masterSelectedIndex = idx"
            >
              <span>{{ getObjectSummary(idx) }}</span>
              <n-button
                text
                type="error"
                size="tiny"
                :disabled="!definition.field.editable"
                @click.stop="removeArrayItem(idx)"
              >
                删除
              </n-button>
            </div>
            <n-button
              size="small"
              :disabled="!definition.field.editable"
              @click="addArrayItem(); masterSelectedIndex = arrayValue.length"
            >
              添加
            </n-button>
          </div>
          <div class="md-detail">
            <template v-if="masterSelectedIndex < arrayValue.length">
              <div
                v-for="fd in objectItemFields"
                :key="fd.key"
                class="md-field"
              >
                <DynamicFormField
                  :definition="{ field: fd, order: 0 }"
                  :component-id="componentId"
                  :model-value="getObjectField(masterSelectedIndex, fd.key)"
                  @update:model-value="(val: unknown) => setObjectField(masterSelectedIndex, fd.key, val)"
                />
              </div>
            </template>
            <n-text v-else depth="3">选择一个项目</n-text>
          </div>
        </div>

        <!-- Object array: Table mode -->
        <div v-else-if="isObjectArray && arrayUiHint === 'Table'" class="array-table-wrap">
          <table class="array-table">
            <thead>
              <tr>
                <th v-for="fd in objectItemFields" :key="fd.key">{{ fd.label }}</th>
                <th v-if="objectItemFields.length > 0 && definition.field.editable" class="col-action">操作</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="(_item, idx) in arrayValue" :key="idx">
                <td v-for="fd in objectItemFields" :key="fd.key">
                  <n-input
                    v-if="isSimpleField(fd, 'Text')"
                    :value="getObjectField(idx, fd.key) as string"
                    size="small"
                    :disabled="!definition.field.editable"
                    @update:value="(val: string) => setObjectField(idx, fd.key, val)"
                  />
                  <n-input-number
                    v-else-if="isFieldNumber(fd)"
                    :value="getObjectField(idx, fd.key) as number"
                    size="small"
                    :min="fieldNumberConfig(fd).min"
                    :max="fieldNumberConfig(fd).max"
                    :step="fieldNumberConfig(fd).step"
                    :disabled="!definition.field.editable"
                    @update:value="(val: number | null) => setObjectField(idx, fd.key, val ?? 0)"
                  />
                  <n-switch
                    v-else-if="isSimpleField(fd, 'Boolean')"
                    :value="getObjectField(idx, fd.key) as boolean"
                    size="small"
                    :disabled="!definition.field.editable"
                    @update:value="(val: boolean) => setObjectField(idx, fd.key, val)"
                  />
                  <n-select
                    v-else-if="isFieldSelect(fd)"
                    :value="getObjectField(idx, fd.key) as string"
                    :options="fieldSelectOptions(fd)"
                    size="small"
                    :disabled="!definition.field.editable"
                    @update:value="(val: string) => setObjectField(idx, fd.key, val)"
                  />
                  <div v-else-if="isFieldPath(fd)" class="path-input-row">
                    <n-input
                      :value="getObjectField(idx, fd.key) as string"
                      size="small"
                      :disabled="!definition.field.editable"
                      @update:value="(val: string) => setObjectField(idx, fd.key, val)"
                    />
                  </div>
                  <span v-else>{{ getObjectField(idx, fd.key) }}</span>
                </td>
                <td v-if="objectItemFields.length > 0 && definition.field.editable" class="col-action">
                  <n-button
                    text
                    type="error"
                    size="tiny"
                    @click="removeArrayItem(idx)"
                  >
                    删除
                  </n-button>
                </td>
              </tr>
            </tbody>
          </table>
          <n-button
            size="small"
            :disabled="!definition.field.editable"
            @click="addArrayItem"
          >
            添加
          </n-button>
        </div>

        <!-- Object array: Default mode (cards) -->
        <div v-else-if="isObjectArray" class="array-object-cards">
          <div
            v-for="(_item, idx) in arrayValue"
            :key="idx"
            class="object-card"
          >
            <div class="object-card-header">
              <span class="card-index">#{{ idx + 1 }}</span>
              <n-button
                text
                type="error"
                size="tiny"
                :disabled="!definition.field.editable"
                @click="removeArrayItem(idx)"
              >
                删除
              </n-button>
            </div>
            <div
              v-for="fd in objectItemFields"
              :key="fd.key"
              class="object-card-field"
            >
              <DynamicFormField
                :definition="{ field: fd, order: 0 }"
                :component-id="componentId"
                :model-value="getObjectField(idx, fd.key)"
                @update:model-value="(val: unknown) => setObjectField(idx, fd.key, val)"
              />
            </div>
          </div>
          <n-button
            size="small"
            :disabled="!definition.field.editable"
            @click="addArrayItem"
          >
            添加
          </n-button>
        </div>
      </div>

      <!-- Unknown / unsupported type fallback -->
      <n-input
        v-else
        :value="JSON.stringify(modelValue)"
        disabled
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'
import { NInput, NInputNumber, NSwitch, NSelect, NColorPicker, NButton, NDynamicTags, NText } from 'naive-ui'
import { open } from '@tauri-apps/plugin-dialog'
import ConfigActionButton from './ConfigActionButton.vue'
import type { ArrayUiHint, FieldDefinition } from '../../bridge/contract'

const props = defineProps<{
  definition: {
    field: FieldDefinition
    group?: string
    order: number
    configAction?: string
  }
  componentId: string
  modelValue: unknown
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: unknown): void
}>()

// ---- type guards ----

function isSimple(type: string): boolean {
  return props.definition.field.settingType === type
}

const isNumber = computed(() => {
  const st = props.definition.field.settingType
  return typeof st === 'object' && st !== null && 'Number' in st
})

const isSelect = computed(() => {
  const st = props.definition.field.settingType
  return typeof st === 'object' && st !== null && 'Select' in st
})

const isPath = computed(() => {
  const st = props.definition.field.settingType
  return typeof st === 'object' && st !== null && 'Path' in st
})

const isArray = computed(() => {
  const st = props.definition.field.settingType
  return typeof st === 'object' && st !== null && 'Array' in st
})

const isInlineType = computed(() =>
  isSimple('Text') || isNumber.value || isPath.value || isSelect.value || isSimple('Color'),
)

// ---- Number config ----

const numberConfig = computed(() => {
  const st = props.definition.field.settingType
  if (typeof st === 'object' && st !== null && 'Number' in st) {
    return (st as { Number: { min?: number; max?: number; step?: number } }).Number
  }
  return { min: undefined, max: undefined, step: undefined } as { min?: number; max?: number; step?: number }
})

// ---- Select options ----

const selectOptions = computed(() => {
  const st = props.definition.field.settingType
  if (typeof st === 'object' && st !== null && 'Select' in st) {
    return (st as { Select: { options: string[] } }).Select.options.map((o) => ({ label: o, value: o }))
  }
  return []
})

// ---- Path ----

const pathPlaceholder = computed(() => {
  const st = props.definition.field.settingType as { Path: { mode: string } } | null
  if (st && 'Path' in st) {
    return st.Path.mode === 'Directory' ? '选择目录...' : '选择文件...'
  }
  return '选择路径...'
})

const pathMode = computed(() => {
  const st = props.definition.field.settingType as { Path: { mode: 'File' | 'Directory' } } | null
  return st?.Path.mode ?? 'File'
})

async function browsePath() {
  try {
    if (pathMode.value === 'Directory') {
      const selected = await open({ directory: true, multiple: false })
      if (selected && typeof selected === 'string') {
        emit('update:modelValue', selected)
      }
    } else {
      const selected = await open({ multiple: false })
      if (selected && typeof selected === 'string') {
        emit('update:modelValue', selected)
      }
    }
  } catch (e) {
    console.error('[DynamicFormField] Path browse failed:', e)
  }
}

// ---- Array ----

const arrayConfig = computed(() => {
  const st = props.definition.field.settingType as { Array: { item: unknown; minItems?: number; maxItems?: number; uiHint: ArrayUiHint } } | null
  return st?.Array ?? null
})

const arrayUiHint = computed<ArrayUiHint>(() => arrayConfig.value?.uiHint ?? 'Default')

const arrayItem = computed(() => arrayConfig.value?.item ?? null)

const isPrimitiveArray = computed(() => {
  const item = arrayItem.value
  return item !== null && typeof item === 'object' && 'Primitive' in item
})

const isObjectArray = computed(() => {
  const item = arrayItem.value
  return item !== null && typeof item === 'object' && 'Object' in item
})

const primitiveItemType = computed(() => {
  if (!isPrimitiveArray.value) return 'Text'
  const item = arrayItem.value as { Primitive: unknown }
  const prim = item.Primitive
  if (typeof prim === 'string') return prim
  if (typeof prim === 'object' && prim !== null) {
    if ('Number' in prim) return 'Number'
    if ('Select' in prim) return 'Select'
    if ('Path' in prim) return 'Path'
    if ('Boolean' in prim) return 'Boolean'
    if ('Color' in prim) return 'Color'
  }
  return 'Text'
})

const numberPrimitiveConfig = computed(() => {
  if (!isPrimitiveArray.value) return {}
  const item = arrayItem.value as { Primitive: { Number: { min?: number; max?: number; step?: number } } }
  if (typeof item.Primitive === 'object' && 'Number' in item.Primitive) {
    return item.Primitive.Number
  }
  return {}
})

const primitiveSelectOptions = computed(() => {
  if (!isPrimitiveArray.value) return []
  const item = arrayItem.value as { Primitive: { Select: { options: string[] } } }
  if (typeof item.Primitive === 'object' && 'Select' in item.Primitive) {
    return item.Primitive.Select.options.map((o) => ({ label: o, value: o }))
  }
  return []
})

const primitivePathMode = computed(() => {
  if (!isPrimitiveArray.value) return 'File'
  const item = arrayItem.value as { Primitive: { Path: { mode: 'File' | 'Directory' } } }
  if (typeof item.Primitive === 'object' && 'Path' in item.Primitive) {
    return item.Primitive.Path.mode
  }
  return 'File'
})

// Object item fields
const objectItemFields = computed<FieldDefinition[]>(() => {
  if (!isObjectArray.value) return []
  const item = arrayItem.value as { Object: FieldDefinition[] }
  return item.Object.filter((f) => f.visible !== false)
})

const arrayValue = computed(() => {
  if (Array.isArray(props.modelValue)) return props.modelValue as unknown[]
  return []
})

function onArrayUpdate(values: unknown) {
  emit('update:modelValue', values)
}

function onArrayItemUpdate(idx: number, val: unknown) {
  const arr = [...arrayValue.value]
  arr[idx] = val
  emit('update:modelValue', arr)
}

function getDefaultItem() {
  if (isPrimitiveArray.value) {
    const def = props.definition.field.defaultValue
    if (Array.isArray(def) && def.length > 0) return def[0]
    switch (primitiveItemType.value) {
      case 'Number': return 0
      case 'Boolean': return false
      default: return ''
    }
  }
  if (isObjectArray.value) {
    const obj: Record<string, unknown> = {}
    for (const fd of objectItemFields.value) {
      obj[fd.key] = fd.defaultValue ?? ''
    }
    return obj
  }
  return ''
}

function addArrayItem() {
  const arr = [...arrayValue.value]
  arr.push(getDefaultItem())
  emit('update:modelValue', arr)
}

function removeArrayItem(idx: number) {
  const arr = [...arrayValue.value]
  arr.splice(idx, 1)
  if (arr.length === 0) {
    masterSelectedIndex.value = 0
  } else if (idx < masterSelectedIndex.value) {
    masterSelectedIndex.value -= 1
  } else if (idx === masterSelectedIndex.value && masterSelectedIndex.value >= arr.length) {
    masterSelectedIndex.value = arr.length - 1
  }
  emit('update:modelValue', arr)
}

// ---- Object array helpers ----

function getObjectField(idx: number, key: string): unknown {
  const item = arrayValue.value[idx]
  if (item && typeof item === 'object') {
    return (item as Record<string, unknown>)[key]
  }
  return ''
}

function setObjectField(idx: number, key: string, val: unknown) {
  const arr = [...arrayValue.value]
  const item = arr[idx]
  if (item && typeof item === 'object') {
    arr[idx] = { ...(item as Record<string, unknown>), [key]: val }
  }
  emit('update:modelValue', arr)
}

const masterSelectedIndex = ref(0)

function getObjectSummary(idx: number): string {
  const item = arrayValue.value[idx]
  if (!item || typeof item !== 'object') return `#${idx + 1}`
  const fields = objectItemFields.value
  if (fields.length === 0) return `#${idx + 1}`
  const firstVal = (item as Record<string, unknown>)[fields[0].key]
  return String(firstVal ?? `#${idx + 1}`)
}

// ---- Array path browsing ----

async function browseArrayPath(idx: number) {
  try {
    if (primitivePathMode.value === 'Directory') {
      const selected = await open({ directory: true, multiple: false })
      if (selected && typeof selected === 'string') {
        onArrayItemUpdate(idx, selected)
      }
    } else {
      const selected = await open({ multiple: false })
      if (selected && typeof selected === 'string') {
        onArrayItemUpdate(idx, selected)
      }
    }
  } catch (e) {
    console.error('[DynamicFormField] Array path browse failed:', e)
  }
}

// ---- Field type helpers for object fields ----

function isSimpleField(fd: FieldDefinition, type: string): boolean {
  return fd.settingType === type
}

function isFieldNumber(fd: FieldDefinition): boolean {
  return typeof fd.settingType === 'object' && fd.settingType !== null && 'Number' in fd.settingType
}

function isFieldSelect(fd: FieldDefinition): boolean {
  return typeof fd.settingType === 'object' && fd.settingType !== null && 'Select' in fd.settingType
}

function isFieldPath(fd: FieldDefinition): boolean {
  return typeof fd.settingType === 'object' && fd.settingType !== null && 'Path' in fd.settingType
}

function fieldNumberConfig(fd: FieldDefinition) {
  if (typeof fd.settingType === 'object' && fd.settingType !== null && 'Number' in fd.settingType) {
    return (fd.settingType as { Number: { min?: number; max?: number; step?: number } }).Number
  }
  return { min: undefined, max: undefined, step: undefined } as { min?: number; max?: number; step?: number }
}

function fieldSelectOptions(fd: FieldDefinition) {
  if (typeof fd.settingType === 'object' && fd.settingType !== null && 'Select' in fd.settingType) {
    return (fd.settingType as { Select: { options: string[] } }).Select.options.map((o) => ({ label: o, value: o }))
  }
  return []
}

// ---- Json ----

const jsonString = computed(() => {
  if (props.modelValue == null) return ''
  try {
    return JSON.stringify(props.modelValue, null, 2)
  } catch {
    return String(props.modelValue)
  }
})

function onJsonInput(val: string) {
  try {
    emit('update:modelValue', JSON.parse(val))
  } catch {
    // Invalid JSON, silently ignore
  }
}
</script>

<style scoped>
.form-field {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.field-label {
  font-size: var(--font-size-sm);
  color: var(--text-primary);
}

.field-desc {
  color: var(--text-secondary);
  font-weight: 400;
}

.field-control {
  display: flex;
  flex-direction: column;
}

.field-input-row {
  display: flex;
  gap: 8px;
  align-items: center;
}

.field-input-row > :first-child {
  flex: 1;
}

.path-input-row {
  display: flex;
  gap: 8px;
  align-items: center;
  flex: 1;
}

.path-input-row > .n-input {
  flex: 1;
}

.path-input-row.array-path {
  flex: 1;
}

/* ---- Arrays ---- */

.array-field {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.array-default {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.array-default .array-row {
  display: flex;
  gap: 6px;
  align-items: center;
}

.array-row > :first-child {
  flex: 1;
}

.array-tags {
  width: 100%;
}

/* ---- Object cards ---- */

.array-object-cards {
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

/* ---- Table ---- */

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

/* ---- MasterDetail ---- */

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
