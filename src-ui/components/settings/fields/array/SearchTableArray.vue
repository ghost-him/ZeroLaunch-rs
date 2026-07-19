<template>
  <div class="search-table-array">
    <!-- 搜索栏 -->
    <div class="search-bar">
      <n-input
        v-model:value="query"
        placeholder="搜索程序..."
        clearable
        :loading="searching"
        @update:value="onSearchInput"
      />
      <n-button @click="doSearch(query)">搜索</n-button>
    </div>

    <!-- 搜索结果表格 -->
    <n-data-table
      v-if="searchResults.length > 0 || query.length > 0"
      :columns="columns"
      :data="searchResults"
      :bordered="false"
      :single-line="false"
      size="small"
      :max-height="400"
    />

    <n-empty v-else-if="!searching" description="输入关键词搜索程序" />

    <!-- 编辑弹窗 -->
    <n-modal
      v-model:show="showModal"
      :title="editingTarget"
      preset="card"
      style="width: 500px"
      :mask-closable="false"
    >
      <div v-for="fd in visibleFields" :key="fd.key" class="modal-field">
        <DynamicFormField
          :definition="{ field: fd, order: 0 }"
          :component-id="componentId"
          :model-value="editingValues[fd.key]"
          @update:model-value="(val: unknown) => { editingValues[fd.key] = val }"
        />
      </div>
      <template #footer>
        <n-space justify="end">
          <n-button size="small" @click="showModal = false">取消</n-button>
          <n-button size="small" type="primary" :loading="saving" :disabled="saving" @click="onSaveEdit">保存</n-button>
        </n-space>
      </template>
    </n-modal>
  </div>
</template>

<script setup lang="ts">
import { computed, h, onMounted, reactive, ref } from 'vue'
import {
  NButton,
  NDataTable,
  NEmpty,
  NInput,
  NModal,
  NSpace,
  NText,
  useMessage,
} from 'naive-ui'
import type { DataTableColumn } from 'naive-ui'
import DynamicFormField from '../../DynamicFormField.vue'
import { configExecuteAction } from '../../../../bridge/commands'
import { getVisibleObjectFields, getSearchTableSource, getSearchTableFieldMapping } from '../../../../utils/schemaTypes'
import type { SettingDefinition, ArrayItem, CandidateSummary } from '../../../../bridge/contract'

const props = defineProps<{
  definition: SettingDefinition
  componentId: string
  modelValue: unknown
  item: ArrayItem
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: unknown): void
}>()

const message = useMessage()

// ---- 从 schema 中提取 SearchTable 源信息 ----
const searchSource = computed(() => {
  const st = props.definition.field.settingType
  if (typeof st === 'object' && st !== null && 'array' in st) {
    return getSearchTableSource(st.array.uiHint)
  }
  return null
})

const visibleFields = computed(() => getVisibleObjectFields(props.item))

// ---- 搜索状态 ----
const query = ref('')
const searchResults = ref<CandidateSummary[]>([])
const searching = ref(false)
let debounceTimer: ReturnType<typeof setTimeout> | null = null

function onSearchInput() {
  if (debounceTimer) clearTimeout(debounceTimer)
  if (!query.value) {
    searchResults.value = []
    return
  }
  debounceTimer = setTimeout(() => doSearch(query.value), 300)
}

// 挂载时自动搜索一次，初始加载显示所有候选项
onMounted(() => doSearch(''))

async function doSearch(q: string) {
  const source = searchSource.value
  if (!source) return
  searching.value = true
  try {
    const result = await configExecuteAction(source.sourceComponent, source.sourceAction, { query: q })
    if (Array.isArray(result)) {
      searchResults.value = result as CandidateSummary[]
    }
  } catch {
    searchResults.value = []
  } finally {
    searching.value = false
  }
}

// ---- 条目管理 ----
const entries = computed<Record<string, unknown>[]>(() => {
  if (Array.isArray(props.modelValue)) return props.modelValue as Record<string, unknown>[]
  return []
})

function getEntry(target: string): Record<string, unknown> | undefined {
  const lowerTarget = target.toLowerCase()
  return entries.value.find((e) => String(e.target ?? '').toLowerCase() === lowerTarget)
}

function updateEntries(newEntries: Record<string, unknown>[]) {
  emit('update:modelValue', newEntries)
}

// ---- 编辑弹窗 ----
const showModal = ref(false)
const editingTarget = ref('')
const saving = ref(false)
const editingValues = reactive<Record<string, unknown>>({})

function onEdit(candidate: CandidateSummary) {
  // 清空旧编辑状态，防止跨编辑会话残留旧属性
  Object.keys(editingValues).forEach(k => delete editingValues[k])
  editingTarget.value = candidate.name
  const entry = getEntry(candidate.target)
  editingValues.target = candidate.target

  // schema 驱动的字段映射：将候选项结果字段自动注入到编辑表单
  const st = props.definition.field.settingType
  const uiHint = typeof st === 'object' && st !== null && 'array' in st ? st.array.uiHint : null
  const mapping = uiHint ? getSearchTableFieldMapping(uiHint) : null
  if (mapping) {
    for (const [candidateField, formField] of mapping) {
      const val = (candidate as unknown as Record<string, unknown>)[candidateField]
      if (val !== undefined && val !== null) {
        editingValues[formField] = val
      }
    }
  }

  for (const fd of visibleFields.value) {
    editingValues[fd.key] = entry?.[fd.key] ?? fd.defaultValue
  }
  showModal.value = true
}

async function onSaveEdit() {
  saving.value = true
  try {
    const target = String(editingValues.target ?? '')

    // 处理 transient 字段：有 configAction 的字段在保存时触发对应动作后丢弃
    for (const fd of visibleFields.value) {
      if (fd.configAction) {
        const val = editingValues[fd.key]
        if (val !== '' && val !== undefined && val !== null) {
          try {
            await configExecuteAction(props.componentId, fd.configAction, {
              ...editingValues,
            })
          } catch (e) {
            console.error(`ConfigAction ${fd.configAction} 失败:`, e)
            message.error(`操作 "${fd.label ?? fd.configAction}" 执行失败，请重试`)
          }
        }
      }
    }

    const newEntries = [...entries.value]
    const lowerTarget = target.toLowerCase()
    const existingIdx = newEntries.findIndex(
      (e) => String(e.target ?? '').toLowerCase() === lowerTarget,
    )

    // 构建保存的条目（排除 configAction 字段）
    const savedEntry: Record<string, unknown> = { target }
    for (const fd of visibleFields.value) {
      if (fd.configAction) continue
      const val = editingValues[fd.key]
      // 跳过空值，避免存储无意义的默认值
      if (val !== '' && val !== undefined && !(Array.isArray(val) && val.length === 0)) {
        savedEntry[fd.key] = val
      }
    }
    // 保存条目（即使只有 target 也需要保存，用于标识已覆盖的项）
    if (existingIdx >= 0) {
      newEntries[existingIdx] = savedEntry
    } else {
      newEntries.push(savedEntry)
    }

    updateEntries(newEntries)
    showModal.value = false
  } finally {
    saving.value = false
  }
}

function onDelete(candidate: CandidateSummary) {
  const lowerTarget = candidate.target.toLowerCase()
  const newEntries = entries.value.filter(
    (e) => String(e.target ?? '').toLowerCase() !== lowerTarget,
  )
  updateEntries(newEntries)
}
// ---- 通用摘要列：根据 schema 的第一个 visible 字段动态生成 ----
const primaryField = computed(() => visibleFields.value[0] ?? null)

function getSummaryValue(entry: Record<string, unknown> | undefined): string {
  if (!entry) return '—'
  const field = primaryField.value
  if (!field) return '—'
  const val = entry[field.key]
  if (Array.isArray(val) && (val as unknown[]).length > 0)
    return (val as unknown[]).join(', ')
  if (val !== undefined && val !== null && val !== '')
    return String(val)
  return '—'
}

const columns = computed<DataTableColumn<CandidateSummary>[]>(() => [
  {
    title: '',
    key: 'icon',
    width: 40,
    render(row: CandidateSummary) {
      if (row.icon) {
        return h('img', {
          src: row.icon,
          style: { width: '24px', height: '24px', objectFit: 'contain' },
        })
      }
      return h('div', { style: { width: '24px', height: '24px' } })
    },
  },
  {
    title: '程序名',
    key: 'name',
    ellipsis: { tooltip: true },
  },
  {
    title: '路径',
    key: 'target',
    ellipsis: { tooltip: true },
  },
  {
    title: primaryField.value?.label ?? '值',
    key: 'summary',
    width: 150,
    render(row: CandidateSummary) {
      const entry = getEntry(row.target)
      return h(NText, { depth: entry ? undefined : 3 }, {
        default: () => getSummaryValue(entry),
      })
    },
  },
  {
    title: '操作',
    key: 'actions',
    width: 120,
    render(row: CandidateSummary) {
      return h('div', { style: { display: 'flex', gap: '4px' } }, [
        h(
          NButton,
          {
            size: 'tiny',
            onClick: () => onEdit(row),
          },
          { default: () => '编辑' },
        ),
        getEntry(row.target)
          ? h(
              NButton,
              {
                size: 'tiny',
                type: 'error',
                quaternary: true,
                onClick: () => onDelete(row),
              },
              { default: () => '删除' },
            )
          : null,
      ])
    },
  },
])
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
