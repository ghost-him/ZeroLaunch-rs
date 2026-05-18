<template>
  <div class="detail-preview-panel">
    <div class="preview-header">
      <span class="preview-title">{{ title }}</span>
      <span class="preview-count">{{ enrichedItems.length }} / {{ previewItems.length }}</span>
    </div>

    <n-input
      v-model:value="filterText"
      placeholder="搜索..."
      clearable
      size="small"
      class="preview-filter"
    />

    <div v-if="loading" class="preview-loading">
      <n-spin :size="16" />
    </div>

    <div v-else-if="errorMsg" class="preview-error">
      <n-text type="error" depth="3">{{ errorMsg }}</n-text>
    </div>

    <n-scrollbar v-else class="preview-list" :style="{ maxHeight: '300px' }">
      <div
        v-for="item in enrichedItems"
        :key="item.key"
        class="preview-item"
        :class="{ excluded: item.excluded }"
      >
        <n-checkbox
          :checked="!item.excluded"
          @update:checked="(checked: boolean) => toggleExcluded(item.key, !checked)"
        />
        <div class="preview-item-content">
          <div class="preview-item-label">
            {{ item.displayLabel }}
            <n-tag v-if="item.hasCustomTitle" size="tiny" :bordered="false" type="info">
              自定义
            </n-tag>
          </div>
          <div class="preview-item-key">{{ item.key }}</div>
        </div>
        <n-button text size="tiny" @click="openEdit(item)">
          编辑
        </n-button>
      </div>

      <div v-if="enrichedItems.length === 0 && previewItems.length > 0" class="preview-empty">
        无匹配结果
      </div>
      <div v-if="previewItems.length === 0 && !loading" class="preview-empty">
        暂无数据
      </div>
    </n-scrollbar>

    <n-modal v-model:show="editDialogVisible" preset="dialog" title="编辑覆盖" style="width: 450px;">
      <div v-if="editingItem" class="edit-form">
        <div class="edit-row">
          <span class="edit-label">URL:</span>
          <n-text>{{ editingItem.key }}</n-text>
        </div>
        <div class="edit-row">
          <span class="edit-label">原始标题:</span>
          <n-text>{{ editingItem.label }}</n-text>
        </div>
        <div class="edit-row">
          <span class="edit-label">自定义标题:</span>
          <n-input v-model:value="editCustomTitle" placeholder="留空则使用原始标题" clearable />
        </div>
        <div class="edit-row">
          <span class="edit-label">排除:</span>
          <n-switch v-model:value="editExcluded" />
        </div>
      </div>
      <template #action>
        <n-button @click="editDialogVisible = false">取消</n-button>
        <n-button type="primary" @click="saveEdit">保存</n-button>
      </template>
    </n-modal>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, inject, onBeforeUnmount } from 'vue'
import {
  NInput, NScrollbar, NCheckbox, NButton, NTag, NText, NSpin, NModal, NSwitch
} from 'naive-ui'
import { useConfigStore } from '../../../../stores/config-store'
import { FORM_VALUES_KEY } from '../../../../utils/formInjection'
import type { DetailActionDef } from '../../../../bridge/contract'

const props = defineProps<{
  componentId: string
  detailAction: DetailActionDef
  paramValue: string
  title?: string
}>()

const configStore = useConfigStore()
const formValues = inject(FORM_VALUES_KEY)!

interface PreviewItem {
  key: string
  label: string
}

interface EnrichedItem extends PreviewItem {
  excluded: boolean
  hasCustomTitle: boolean
  displayLabel: string
}

const previewItems = ref<PreviewItem[]>([])
const loading = ref(false)
const errorMsg = ref<string | null>(null)
const filterText = ref('')
const debouncedFilter = ref('')
const overridesVersion = ref(0)

const editDialogVisible = ref(false)
const editingItem = ref<PreviewItem | null>(null)
const editCustomTitle = ref('')
const editExcluded = ref(false)

let filterTimer: ReturnType<typeof setTimeout> | undefined
let watchGeneration = 0

watch(filterText, (val) => {
  clearTimeout(filterTimer)
  filterTimer = setTimeout(() => {
    debouncedFilter.value = val
  }, 200)
})

watch(
  () => props.paramValue,
  async (newPath) => {
    if (!newPath) {
      previewItems.value = []
      return
    }
    const gen = ++watchGeneration
    loading.value = true
    errorMsg.value = null
    filterText.value = ''
    debouncedFilter.value = ''
    try {
      const result = await configStore.executeAction(
        props.componentId,
        props.detailAction.action,
        { [props.detailAction.paramKey]: newPath },
      )
      if (gen !== watchGeneration) return
      if (Array.isArray(result)) {
        previewItems.value = result.map((item: Record<string, unknown>) => ({
          key: String(item[props.detailAction.previewItemKey] ?? ''),
          label: String(item[props.detailAction.previewItemLabel] ?? ''),
        })).filter(item => item.key.length > 0)
      } else {
        previewItems.value = []
      }
    } catch (e) {
      if (gen !== watchGeneration) return
      errorMsg.value = String(e)
      previewItems.value = []
    } finally {
      if (gen === watchGeneration) loading.value = false
    }
  },
  { immediate: true },
)

onBeforeUnmount(() => {
  clearTimeout(filterTimer)
})

function getOverrides(): Record<string, unknown>[] {
  const raw = formValues.getValue(props.detailAction.targetField)
  if (Array.isArray(raw)) return raw as Record<string, unknown>[]
  return []
}

function setOverrides(overrides: Record<string, unknown>[]) {
  formValues.setValue(props.detailAction.targetField, overrides)
  overridesVersion.value++
}

function normalizeKey(key: string): string {
  let k = key.trim()
  if (k.endsWith('/') && !k.endsWith('://')) {
    k = k.slice(0, -1)
  }
  return k.toLowerCase()
}

/// Build override lookup map and enrich preview items with computed display fields.
/// Single pass over overrides (build map) + single pass over filtered previews (enrich).
const enrichedItems = computed<EnrichedItem[]>(() => {
  overridesVersion.value

  const matchKey = props.detailAction.targetMatchKey
  const overrideMap = new Map<string, Record<string, unknown>>()
  for (const o of getOverrides()) {
    overrideMap.set(normalizeKey(String(o[matchKey] ?? '')), o)
  }

  let items = previewItems.value
  const q = debouncedFilter.value.trim()
  if (q) {
    const lq = q.toLowerCase()
    items = items.filter(
      (item) => item.label.toLowerCase().includes(lq) || item.key.toLowerCase().includes(lq),
    )
  }

  return items.map((item) => {
    const override = overrideMap.get(normalizeKey(item.key))
    const excluded = override?.excluded === true
    const customTitle =
      typeof override?.custom_title === 'string' ? String(override.custom_title).trim() : ''
    return {
      ...item,
      excluded,
      hasCustomTitle: customTitle.length > 0,
      displayLabel: customTitle || item.label,
    }
  })
})

function hasCustomTitleInOverride(override: Record<string, unknown>): boolean {
  const title = override.custom_title
  return typeof title === 'string' && title.trim().length > 0
}

function toggleExcluded(key: string, excluded: boolean) {
  const matchKey = props.detailAction.targetMatchKey
  const overrides = [...getOverrides()]
  const nk = normalizeKey(key)
  const idx = overrides.findIndex(
    (o) => normalizeKey(String(o[matchKey] ?? '')) === nk,
  )

  if (idx >= 0) {
    if (!excluded && !hasCustomTitleInOverride(overrides[idx])) {
      overrides.splice(idx, 1)
    } else {
      overrides[idx] = { ...overrides[idx], excluded }
    }
  } else if (excluded) {
    overrides.push({
      [matchKey]: key,
      excluded: true,
      custom_title: '',
    })
  }

  setOverrides(overrides)
}

function openEdit(item: PreviewItem) {
  editingItem.value = item
  const matchKey = props.detailAction.targetMatchKey
  const nk = normalizeKey(item.key)
  const overrides = getOverrides()
  const override = overrides.find(
    (o) => normalizeKey(String(o[matchKey] ?? '')) === nk,
  )
  editCustomTitle.value = (override?.custom_title as string) ?? ''
  editExcluded.value = override?.excluded === true
  editDialogVisible.value = true
}

function saveEdit() {
  if (!editingItem.value) return

  const key = editingItem.value.key
  const matchKey = props.detailAction.targetMatchKey
  const overrides = [...getOverrides()]
  const nk = normalizeKey(key)
  const idx = overrides.findIndex(
    (o) => normalizeKey(String(o[matchKey] ?? '')) === nk,
  )

  const hasChanges = editExcluded.value || editCustomTitle.value.trim().length > 0

  if (idx >= 0) {
    if (!hasChanges) {
      overrides.splice(idx, 1)
    } else {
      overrides[idx] = {
        ...overrides[idx],
        excluded: editExcluded.value,
        custom_title: editCustomTitle.value.trim(),
      }
    }
  } else if (hasChanges) {
    overrides.push({
      [matchKey]: key,
      excluded: editExcluded.value,
      custom_title: editCustomTitle.value.trim(),
    })
  }

  setOverrides(overrides)
  editDialogVisible.value = false
}
</script>

<style scoped>
.detail-preview-panel {
  border-top: 1px solid var(--border-color);
  margin-top: 8px;
  padding-top: 8px;
}
.preview-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 6px;
}
.preview-title {
  font-size: var(--font-size-sm);
  font-weight: 600;
}
.preview-count {
  font-size: 11px;
  color: var(--text-secondary);
}
.preview-filter {
  margin-bottom: 6px;
}
.preview-loading, .preview-error, .preview-empty {
  text-align: center;
  padding: 16px;
  color: var(--text-secondary);
  font-size: var(--font-size-sm);
}
.preview-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 6px;
  font-size: var(--font-size-sm);
  border-bottom: 1px solid var(--border-color);
  transition: opacity 0.2s;
}
.preview-item:hover {
  background: var(--bg-secondary);
}
.preview-item.excluded {
  opacity: 0.5;
}
.preview-item-content {
  flex: 1;
  overflow: hidden;
}
.preview-item-label {
  display: flex;
  align-items: center;
  gap: 4px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.preview-item-key {
  font-size: 11px;
  color: var(--text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.edit-form {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.edit-row {
  display: flex;
  align-items: center;
  gap: 8px;
}
.edit-label {
  min-width: 80px;
  font-size: var(--font-size-sm);
  color: var(--text-secondary);
}
</style>
