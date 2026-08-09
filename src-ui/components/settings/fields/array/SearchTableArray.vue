<template>
  <div class="search-table-array">
    <n-alert v-if="!searchSource" type="warning">
      {{ $t('settings.searchActionMissing') }}
    </n-alert>
    <n-alert v-else-if="searchError" type="warning">
      {{ searchError }}
    </n-alert>

    <div class="search-bar">
      <n-input
        v-model:value="query"
        :placeholder="field.description || $t('search.placeholder')"
        size="small"
        clearable
        :disabled="field.readOnly || !searchSource"
        @update:value="onSearchInput"
        @keydown.enter="doSearch(query)"
      />
      <n-button
        size="small"
        :loading="searching"
        :disabled="field.readOnly || !searchSource"
        @click="doSearch(query)"
      >
        {{ $t('common.search') }}
      </n-button>
    </div>

    <n-data-table
      v-if="searchResults.length > 0 || query.length > 0"
      :columns="columns"
      :data="searchResults"
      :bordered="false"
      :single-line="false"
      size="small"
      :max-height="400"
    />

    <n-empty v-else-if="!searching" :description="$t('search.noResults')" />

    <n-modal
      v-model:show="showModal"
      :title="$t('settings.editEntry', { label: editingTarget })"
      preset="card"
      style="width: 480px"
      :mask-closable="false"
    >
      <div v-for="fd in visibleFields" :key="fd.key" class="modal-field">
        <label>{{ fd.label }}</label>
        <DynamicFormField
          :field="fdToConfig(fd, field.readOnly)"
          :component-id="componentId"
          :model-value="editingValues[fd.key]"
          @update:model-value="(value: unknown) => setEditingValue(fd.key, value)"
        />
      </div>
      <template #footer>
        <n-space justify="end">
          <n-button @click="showModal = false">{{ $t('common.cancel') }}</n-button>
          <n-button
            type="primary"
            :loading="saving"
            :disabled="field.readOnly || !canSaveEdit"
            @click="onSaveEdit"
          >
            {{ $t('common.save') }}
          </n-button>
        </n-space>
      </template>
    </n-modal>
  </div>
</template>

<script setup lang="ts">
import { computed, h, onBeforeUnmount, onMounted, provide, ref, watch } from 'vue'
import {
  NAlert,
  NButton,
  NDataTable,
  NEmpty,
  NInput,
  NModal,
  NSpace,
} from 'naive-ui'
import { useI18n } from 'vue-i18n'
import type { DataTableColumn } from 'naive-ui'
import DynamicFormField from '../../DynamicFormField.vue'
import IconDisplay from '../../../common/IconDisplay.vue'
import { useConfigStore } from '../../../../stores/config-store'
import {
  canAddArrayItem,
  canRemoveArrayItem,
  getArrayItemSchema,
  getObjectFieldDefs,
  fieldDefToConfig,
  isObjectSchema,
} from '../../../../utils/schemaTypes'
import type { FieldConfig } from '../../../../utils/schemaTypes'
import { FORM_VALUES_KEY } from '../../../../utils/formInjection'

type SearchResult = Record<string, unknown>

const props = defineProps<{
  field: FieldConfig
  componentId: string
  modelValue: unknown
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: unknown): void
}>()

const { t } = useI18n()
const configStore = useConfigStore()

const searchSource = computed(() => {
  const action = props.field.action
  if (!action || action.kind !== 'data') return null
  const binding = action.binding
  return {
    sourceComponent: binding.component ?? props.componentId,
    sourceAction: binding.action,
    labelField: binding.labelField,
    labelFieldLabel: binding.labelFieldLabel,
    valueField: binding.valueField,
    fieldMapping: binding.fieldMapping ?? [],
  }
})
/** 将缺失或无效的搜索 action 记录到控制台。 */
watch(searchSource, (source) => {
  if (!source) console.warn(`[settings] ${props.field.key}: search action is missing`)
}, { immediate: true })

const itemSchema = computed(() => getArrayItemSchema(props.field.schema))
const objectItemSchema = computed(() => {
  const schema = itemSchema.value
  return schema && isObjectSchema(schema) ? schema : null
})

const allFields = computed(() => objectItemSchema.value ? getObjectFieldDefs(objectItemSchema.value, true) : [])
const visibleFields = computed(() => allFields.value.filter((field) => field.visible))
const fdToConfig = fieldDefToConfig

const query = ref('')
const searchResults = ref<SearchResult[]>([])

/** 搜索结果是否携带 icon 字段（如候选注册表返回的 base64 图标），决定是否渲染图标列。 */
const hasIconColumn = computed(() =>
  searchResults.value.some((row) => typeof row['icon'] === 'string' && row['icon'].length > 0),
)
const searching = ref(false)
const searchError = ref<string | null>(null)
let debounceTimer: ReturnType<typeof setTimeout> | null = null
/** 记录搜索 action 返回或执行失败的警告。 */
watch(searchError, (error) => {
  if (error) console.warn(`[settings] ${props.field.key}: ${error}`)
})

const entries = computed<Record<string, unknown>[]>(() => {
  if (Array.isArray(props.modelValue)) {
    return props.modelValue.filter(isRecord)
  }
  return []
})

const showModal = ref(false)
const editingTarget = ref('')
const saving = ref(false)
const editingValues = ref<Record<string, unknown>>({})

const canSaveEdit = computed(() => {
  const source = searchSource.value
  if (!source) return false
  const value = editingValues.value[source.valueField]
  return value !== undefined && value !== null && String(value).trim().length > 0
})

/** 判断未知 action 返回值是否为普通对象。 */
function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

/** 从当前 schema 的 valueField 查找已保存条目。 */
function getEntry(value: unknown): Record<string, unknown> | undefined {
  const source = searchSource.value
  if (!source) return undefined
  const normalized = String(value ?? '').toLowerCase()
  return entries.value.find(
    (entry) => String(entry[source.valueField] ?? '').toLowerCase() === normalized,
  )
}

/** 更新编辑弹窗中的字段值并保持响应式引用不可变。 */
function setEditingValue(key: string, value: unknown): void {
  editingValues.value = { ...editingValues.value, [key]: value }
}

provide(FORM_VALUES_KEY, {
  getValue: (key: string) => editingValues.value[key],
  setValue: setEditingValue,
  values: editingValues,
})

/** 在输入变化后延迟执行 schema action 搜索。 */
function onSearchInput(): void {
  if (props.field.readOnly) return
  if (debounceTimer) clearTimeout(debounceTimer)
  if (!query.value) {
    searchResults.value = []
    searchError.value = null
    return
  }
  debounceTimer = setTimeout(() => doSearch(query.value), 300)
}

/** 执行配置 data action，并验证返回值满足 SearchTable 数据契约。 */
async function doSearch(searchQuery: string): Promise<void> {
  const source = searchSource.value
  if (props.field.readOnly) return
  if (!source) return
  const schema = objectItemSchema.value
  if (
    !schema
    || !(source.valueField in schema.properties)
    || source.fieldMapping.some(([, toField]) => !(toField in schema.properties))
  ) {
    searchError.value = t('settings.searchActionInvalid')
    return
  }
  searchError.value = null
  searching.value = true
  try {
    const result = await configStore.executeAction(
      source.sourceComponent,
      source.sourceAction,
      { query: searchQuery },
    )
    if (!Array.isArray(result) || !result.every(isRecord)) {
      searchResults.value = []
      searchError.value = t('settings.searchActionInvalid')
      return
    }
    if (!result.every((row) => {
      const value = row[source.valueField]
      return value !== undefined && value !== null && String(value).trim().length > 0
    })) {
      searchResults.value = []
      searchError.value = t('settings.searchValueRequired')
      return
    }
    searchResults.value = result
  } catch (error) {
    searchResults.value = []
    searchError.value = t('settings.searchActionFailed', { message: String(error) })
  } finally {
    searching.value = false
  }
}

/** 将搜索结果映射到 schema entry 编辑值，并优先保留已有条目。 */
function onEdit(candidate: SearchResult): void {
  if (props.field.readOnly) return
  const source = searchSource.value
  if (!source) return
  const existing = getEntry(candidate[source.valueField])
  const values: Record<string, unknown> = {}
  for (const field of allFields.value) {
    values[field.key] = existing?.[field.key]
  }
  values[source.valueField] = existing?.[source.valueField] ?? candidate[source.valueField]
  for (const [fromField, toField] of source.fieldMapping) {
    if (existing?.[toField] === undefined && candidate[fromField] !== undefined) {
      values[toField] = candidate[fromField]
    }
  }
  editingValues.value = values
  editingTarget.value = String(candidate[source.labelField] ?? candidate[source.valueField] ?? '')
  showModal.value = true
}

/** 保存当前 schema entry，并遵守数组容量与父级 readOnly 约束。 */
function onSaveEdit(): void {
  if (props.field.readOnly || !canSaveEdit.value) return
  const source = searchSource.value
  if (!source || !objectItemSchema.value) return
  const value = editingValues.value[source.valueField]
  const existing = getEntry(value)
  const newEntries = [...entries.value]
  const existingIndex = existing ? newEntries.indexOf(existing) : -1
  if (existingIndex < 0 && !canAddArrayItem(props.field.schema, newEntries.length)) {
    searchError.value = t('settings.searchMaxItems')
    return
  }

  const savedEntry: Record<string, unknown> = {}
  for (const field of allFields.value) {
    if (field.action?.kind === 'effect' && field.action.binding.transient) continue
    const fieldValue = editingValues.value[field.key]
    if (fieldValue !== undefined) savedEntry[field.key] = fieldValue
  }
  if (savedEntry[source.valueField] === undefined) {
    savedEntry[source.valueField] = value
  }
  if (existingIndex >= 0) {
    newEntries[existingIndex] = savedEntry
  } else {
    newEntries.push(savedEntry)
  }
  emit('update:modelValue', newEntries)
  showModal.value = false
}

/** 删除搜索结果对应的 schema entry，并遵守 minItems 约束。 */
function onDelete(candidate: SearchResult): void {
  if (props.field.readOnly || !canRemoveArrayItem(props.field.schema, entries.value.length)) return
  const source = searchSource.value
  if (!source) return
  const value = candidate[source.valueField]
  emit(
    'update:modelValue',
    entries.value.filter((entry) => String(entry[source.valueField] ?? '').toLowerCase() !== String(value ?? '').toLowerCase()),
  )
}


const columns = computed<DataTableColumn<SearchResult>[]>(() => {
  const source = searchSource.value
  if (!source) return []
  const labelField = allFields.value.find((field) => field.key === source.labelField)
  const valueField = allFields.value.find((field) => field.key === source.valueField)
  const resultColumns: DataTableColumn<SearchResult>[] = [
    {
      title: labelField?.label ?? (source.labelFieldLabel || source.labelField),
      key: 'label',
      ellipsis: { tooltip: true },
      render: (row) => String(row[source.labelField] ?? row[source.valueField] ?? '—'),
    },
  ]
  if (hasIconColumn.value) {
    resultColumns.unshift({
      title: t('settings.icon'),
      key: 'icon',
      width: 60,
      render: (row) => {
        const src = typeof row['icon'] === 'string' ? row['icon'] : ''
        return src ? h(IconDisplay, { src, size: 28 }) : null
      },
    })
  }
  if (source.valueField !== source.labelField) {
    resultColumns.push({
      title: valueField?.label ?? source.valueField,
      key: 'value',
      ellipsis: { tooltip: true },
      render: (row) => String(row[source.valueField] ?? '—'),
    })
  }
  resultColumns.push({
    title: t('common.actions'),
    key: 'actions',
    width: 80,
    render: (row) => {
      const value = row[source.valueField]
      const existing = getEntry(value)
      return h('div', { style: { display: 'flex', gap: '4px' } }, [
        h(NButton, {
          size: 'tiny',
          disabled: props.field.readOnly,
          onClick: () => onEdit(row),
        }, { default: () => t('common.edit') }),
        existing && !props.field.readOnly
          ? h(NButton, {
              size: 'tiny',
              type: 'error',
              quaternary: true,
              disabled: !canRemoveArrayItem(props.field.schema, entries.value.length),
              onClick: () => onDelete(row),
            }, { default: () => t('common.delete') })
          : null,
      ])
    },
  })
  return resultColumns
})

onMounted(() => {
  if (searchSource.value && !props.field.readOnly) void doSearch('')
})

onBeforeUnmount(() => {
  if (debounceTimer) clearTimeout(debounceTimer)
})
</script>

<style scoped>
.search-table-array {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.search-bar {
  display: flex;
  gap: 8px;
  align-items: center;
}
.search-bar .n-input {
  flex: 1;
}
.modal-field {
  margin-bottom: 12px;
}
</style>
