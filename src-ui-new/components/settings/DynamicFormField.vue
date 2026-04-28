<template>
  <div class="form-field" v-if="definition.field.visible">
    <label class="field-label">
      {{ definition.field.label }}
      <span class="field-desc" v-if="definition.field.description"> — {{ definition.field.description }}</span>
    </label>

    <div class="field-control">
      <!-- ========== Inline types: input + optional config action button ========== -->
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
          @update:value="$emit('update:modelValue', $event)"
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

      <!-- ========== Boolean: switch ========== -->
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

      <!-- ========== Json: textarea ========== -->
      <div v-else-if="isSimple('Json')">
        <n-input
          type="textarea"
          :value="jsonString"
          :disabled="!definition.field.editable"
          :autosize="{ minRows: 2, maxRows: 6 }"
          @update:value="onJsonInput"
        />
      </div>

      <!-- ========== Array types ========== -->
      <div v-else-if="isArray" class="array-field">
        <!-- Primitive tags mode -->
        <div v-if="isPrimitiveArray && arrayUiHint === 'Tags'" class="array-tags">
          <n-dynamic-tags
            :value="arrayValue as string[]"
            :disabled="!definition.field.editable"
            @update:value="onArrayUpdate"
          />
        </div>

        <!-- Primitive default mode: simple list with add/remove -->
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
              v-else-if="isNumberPrimitive"
              :value="item as number"
              :min="numberPrimitiveConfig.min"
              :max="numberPrimitiveConfig.max"
              :step="numberPrimitiveConfig.step"
              :disabled="!definition.field.editable"
              size="small"
              @update:value="(val: number | null) => onArrayItemUpdate(idx, val)"
            />
            <n-switch
              v-else-if="primitiveItemType === 'Boolean'"
              :value="item as boolean"
              :disabled="!definition.field.editable"
              @update:value="(val: boolean) => onArrayItemUpdate(idx, val)"
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

        <!-- TODO: Object array / Table / MasterDetail — not yet implemented -->
        <div v-else class="array-unsupported">
          <n-text depth="3">
            暂不支持此数组类型（{{ arrayUiHint }} / Object），已回退为 JSON 编辑。
          </n-text>
          <n-input
            type="textarea"
            :value="jsonString"
            :disabled="!definition.field.editable"
            :autosize="{ minRows: 2, maxRows: 6 }"
            @update:value="onJsonInput"
          />
        </div>
      </div>

      <!-- ========== Unknown / unsupported type fallback ========== -->
      <n-input
        v-else
        :value="JSON.stringify(modelValue)"
        disabled
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { NInput, NInputNumber, NSwitch, NSelect, NColorPicker, NButton, NDynamicTags, NText } from 'naive-ui'
import { open } from '@tauri-apps/plugin-dialog'
import ConfigActionButton from './ConfigActionButton.vue'
import type { SettingType, ArrayUiHint } from '../../bridge/contract'

interface FieldDefinitionProps {
  key: string
  label: string
  description: string
  settingType: SettingType
  defaultValue: unknown
  visible: boolean
  editable: boolean
}

const props = defineProps<{
  definition: {
    field: FieldDefinitionProps
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
    if ('Select' in prim) return 'Text' // simplified
    if ('Path' in prim) return 'Text'   // simplified
    if ('Boolean' in prim) return 'Boolean'
  }
  return 'Text'
})

const isNumberPrimitive = computed(() => primitiveItemType.value === 'Number')

const numberPrimitiveConfig = computed(() => {
  if (!isPrimitiveArray.value) return {}
  const item = arrayItem.value as { Primitive: { Number: { min?: number; max?: number; step?: number } } }
  if (typeof item.Primitive === 'object' && 'Number' in item.Primitive) {
    return item.Primitive.Number
  }
  return {}
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

function addArrayItem() {
  const arr = [...arrayValue.value]
  const def = props.definition.field.defaultValue
  const itemDefault = Array.isArray(def) && def.length > 0 ? def[0] : ''
  arr.push(itemDefault)
  emit('update:modelValue', arr)
}

function removeArrayItem(idx: number) {
  const arr = [...arrayValue.value]
  arr.splice(idx, 1)
  emit('update:modelValue', arr)
}

onMounted(() => {
  if (isArray.value && !isPrimitiveArray.value) {
    console.warn(
      `[DynamicFormField] Unsupported array type for field "${props.definition.field.key}": ` +
      `uiHint=${arrayUiHint.value}, itemType=${isObjectArray.value ? 'Object' : 'unknown'}. ` +
      `Falling back to JSON editor.`,
    )
  }
})

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

.array-row {
  display: flex;
  gap: 6px;
  align-items: center;
}

.array-row > :first-child {
  flex: 1;
}

.array-unsupported {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.array-tags {
  width: 100%;
}
</style>
