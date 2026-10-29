<template>
  <div ref="rootEl" class="array-master-detail" :class="{ 'drop-hover': isHovered }">
    <div class="md-list">
      <div
        v-for="(item, idx) in listValue"
        :key="rowKey(item, idx)"
        class="md-list-item"
        :class="{ active: selectedIndex === idx }"
        @click="selectedIndex = idx"
      >
        <span>{{ summary(idx) }}</span>
        <n-button
          text type="error" size="tiny"
          :disabled="field.readOnly || !canRemoveArrayItem(field.schema, listValue.length)"
          @click.stop="onRemove(idx)"
        >
          {{ $t('common.delete') }}
        </n-button>
      </div>
      <n-button
        size="small"
        :disabled="field.readOnly || !canAddArrayItem(field.schema, listValue.length)"
        @click="onAdd"
      >
        {{ $t('common.add') }}
      </n-button>
      <n-text v-if="isHovered" depth="3" class="drop-hint">
        {{ $t('settings.drop.addEntries') }}
      </n-text>
    </div>
    <div class="md-detail">
      <template v-if="selectedIndex < listValue.length">
        <div v-for="fd in visibleSubFields" :key="fd.key" class="md-field">
          <DynamicFormField
            :field="fdToConfig(fd, field.readOnly)"
            :component-id="componentId"
            :model-value="getField(selectedIndex, fd.key)"
            @update:model-value="(val: unknown) => setField(selectedIndex, fd.key, val)"
          />
        </div>
        <DetailPreviewPanel
          v-if="field.detailAction && selectedParamValue"
          :component-id="componentId"
          :detail-action="field.detailAction"
          :param-value="selectedParamValue"
          :read-only="field.readOnly"
        />
      </template>
      <n-text v-else depth="3">{{ $t('common.selectItem') }}</n-text>
    </div>
  </div>
</template>
<script setup lang="ts">
import { ref, computed } from 'vue'
import { NButton, NText, useMessage } from 'naive-ui'
import { useI18n } from 'vue-i18n'
import DynamicFormField from '../../DynamicFormField.vue'
import DetailPreviewPanel from './DetailPreviewPanel.vue'
import { useFileDropTarget, type DroppedPath } from '../../../../composables/useFileDrop'
import {
  canAddArrayItem,
  canRemoveArrayItem,
  filterVisibleByContext,
  getArrayItemSchema,
  getDefaultArrayItem,
  getObjectFieldDefs,
  fieldDefToConfig,
} from '../../../../utils/schemaTypes'
import type { FieldConfig } from '../../../../utils/schemaTypes'

const props = defineProps<{
  field: FieldConfig
  componentId: string
  modelValue: unknown
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: unknown): void
}>()

const message = useMessage()
const { t } = useI18n()
const rootEl = ref<HTMLElement | null>(null)

const itemSchema = computed(() => getArrayItemSchema(props.field.schema))

const subFields = computed(() => itemSchema.value ? getObjectFieldDefs(itemSchema.value) : [])
const fdToConfig = fieldDefToConfig
const selectedIndex = ref(0)

/** 当前选中条目的动态可见字段（visibleWhen 按条目自身值过滤）。 */
const visibleSubFields = computed(() =>
  filterVisibleByContext(subFields.value, itemValue(selectedIndex.value)),
)

/** 读取指定条目的值对象；缺失时返回空对象。 */
function itemValue(idx: number): Record<string, unknown> {
  const item = listValue.value[idx]
  if (item && typeof item === 'object' && !Array.isArray(item)) {
    return item as Record<string, unknown>
  }
  return {}
}

const selectedParamValue = computed<string | undefined>(() => {
  const da = props.field.detailAction
  if (!da) return undefined
  const item = listValue.value[selectedIndex.value]
  if (!item || typeof item !== 'object' || Array.isArray(item)) return undefined
  const val = (item as Record<string, unknown>)[da.paramField]
  return typeof val === 'string' && val.length > 0 ? val : undefined
})

const listValue = computed<unknown[]>(() => {
  if (Array.isArray(props.modelValue)) return props.modelValue as unknown[]
  return []
})

/** 读取主从条目字段值；缺失值保持 undefined，由后端决定最终语义。 */
function getField(idx: number, key: string): unknown {
  const item = listValue.value[idx]
  if (item && typeof item === 'object' && !Array.isArray(item)) {
    return (item as Record<string, unknown>)[key]
  }
  return undefined
}

/** 更新主从条目字段并保持父级只读约束。 */
function setField(idx: number, key: string, val: unknown): void {
  if (props.field.readOnly) return
  const arr = [...listValue.value]
  const item = arr[idx]
  if (item && typeof item === 'object' && !Array.isArray(item)) {
    arr[idx] = { ...(item as Record<string, unknown>), [key]: val }
  }
  emit('update:modelValue', arr)
}

/** 生成左侧条目的摘要文本。 */
function summary(idx: number): string {
  const item = listValue.value[idx]
  if (!item || typeof item !== 'object' || Array.isArray(item)) return `#${idx + 1}`
  const firstField = subFields.value[0]
  if (!firstField) return `#${idx + 1}`
  return String((item as Record<string, unknown>)[firstField.key] ?? `#${idx + 1}`)
}

/** v-for 稳定 key：优先取行内首个字段值（如 name/id），避免索引 key 在中间删除时
 *  子字段组件（含内部输入态）串位；值可能重复/变更时退化为索引（与删除串位无关）。 */
function rowKey(item: unknown, idx: number): string {
  if (item && typeof item === 'object' && !Array.isArray(item)) {
    const firstKey = subFields.value[0]?.key
    if (firstKey !== undefined) {
      const v = (item as Record<string, unknown>)[firstKey]
      if (typeof v === 'string' || typeof v === 'number') return `${v}`
    }
  }
  return `idx-${idx}`
}

/** 添加一个符合 item schema 默认值的条目。 */
function onAdd(): void {
  if (!canAddArrayItem(props.field.schema, listValue.value.length) || props.field.readOnly || !itemSchema.value) return
  const arr = [...listValue.value]
  arr.push(getDefaultArrayItem(itemSchema.value))
  selectedIndex.value = arr.length - 1
  emit('update:modelValue', arr)
}

/** 删除条目并遵守 minItems 约束。 */
function onRemove(idx: number): void {
  if (!canRemoveArrayItem(props.field.schema, listValue.value.length) || props.field.readOnly) return
  const arr = [...listValue.value]
  arr.splice(idx, 1)
  selectedIndex.value = Math.min(selectedIndex.value, Math.max(0, arr.length - 1))
  emit('update:modelValue', arr)
}

/** 归一化路径用于去重：统一分隔符、去尾分隔符、忽略大小写（Windows 语义）。 */
function normalizePath(path: string): string {
  return path.replace(/\//g, '\\').replace(/\\+$/, '').toLowerCase()
}

/** 取路径的父目录；根路径（如 `C:\`）没有父目录时返回空串。 */
function parentDirectory(path: string): string {
  const trimmed = path.replace(/[\\/]+$/, '')
  const index = Math.max(trimmed.lastIndexOf('\\'), trimmed.lastIndexOf('/'))
  return index > 0 ? trimmed.slice(0, index) : ''
}

/** 取文件扩展名通配符（`*.exe`）；无扩展名时返回 null。 */
function extensionPattern(path: string): string | null {
  const name = path.slice(Math.max(path.lastIndexOf('\\'), path.lastIndexOf('/')) + 1)
  const dot = name.lastIndexOf('.')
  if (dot <= 0 || dot === name.length - 1) return null
  return `*.${name.slice(dot + 1).toLowerCase()}`
}

/** 读取条目子字段的字符串数组；缺失或类型不符时返回空数组。 */
function stringArray(value: unknown): string[] {
  return Array.isArray(value) ? value.filter((item): item is string => typeof item === 'string') : []
}

/**
 * 处理拖入路径：目录追加为新条目；文件挂到父目录条目并把 `*.ext` 并入模式子字段。
 * 参数：`paths` — 拖入项（含宿主判定的文件/目录类型）。
 */
function applyDroppedPaths(paths: DroppedPath[]): void {
  const spec = props.field.drop
  const schema = itemSchema.value
  if (!spec || !schema || props.field.readOnly) return

  const next = [...listValue.value]
  const patternField = spec.patternField
  let added = 0
  let skipped = 0
  let updated = 0
  let unreadable = 0
  let limited = false
  let firstAddedIndex = -1

  for (const entry of paths) {
    if (entry.kind === 'unavailable') {
      unreadable += 1
      continue
    }
    const targetPath = entry.kind === 'directory' ? entry.path : parentDirectory(entry.path)
    if (!targetPath) {
      unreadable += 1
      continue
    }
    const pattern = entry.kind === 'file' ? extensionPattern(entry.path) : null
    const normalized = normalizePath(targetPath)
    const existing = next.findIndex((item) =>
      item !== null && typeof item === 'object' && !Array.isArray(item)
      && normalizePath(String((item as Record<string, unknown>)[spec.pathField] ?? '')) === normalized,
    )

    if (existing >= 0) {
      if (entry.kind !== 'file' || !patternField || !pattern) {
        skipped += 1
        continue
      }
      const item = { ...(next[existing] as Record<string, unknown>) }
      const patterns = stringArray(item[patternField])
      if (patterns.some((value) => value.toLowerCase() === pattern)) {
        skipped += 1
        continue
      }
      item[patternField] = [...patterns, pattern]
      next[existing] = item
      updated += 1
      continue
    }

    if (!canAddArrayItem(props.field.schema, next.length)) {
      limited = true
      break
    }
    const item: Record<string, unknown> = {
      ...(getDefaultArrayItem(schema) as Record<string, unknown>),
      [spec.pathField]: targetPath,
    }
    if (patternField && pattern) {
      item[patternField] = [...stringArray(item[patternField]), pattern]
    }
    if (firstAddedIndex < 0) firstAddedIndex = next.length
    next.push(item)
    added += 1
  }

  if (added === 0 && updated === 0) {
    if (limited) message.warning(t('settings.drop.limitReached'))
    else if (skipped > 0) message.warning(t('settings.drop.skipped', { count: skipped }))
    else if (unreadable > 0) message.warning(t('settings.drop.unreadable', { count: unreadable }))
    return
  }

  if (firstAddedIndex >= 0) selectedIndex.value = firstAddedIndex
  emit('update:modelValue', next)

  if (limited) message.warning(t('settings.drop.limitReached'))
  else if (added > 0) message.success(t('settings.drop.added', { count: added }))
  else message.success(t('settings.drop.updated', { count: updated }))
}

/** 拖放投放：数组卡片整体作为目标，命中后追加条目或合并模式。 */
const { isHovered } = useFileDropTarget(rootEl, {
  enabled: Boolean(props.field.drop) && !props.field.readOnly,
  onDrop: applyDroppedPaths,
})
</script>

<style scoped>
.array-master-detail {
  display: flex;
  gap: 12px;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  overflow: hidden;
  min-width: 0;
}
.array-master-detail.drop-hover {
  border-color: var(--primary-color);
}
.drop-hint {
  font-size: var(--font-size-sm);
  color: var(--primary-color);
}
.md-list {
  width: 200px;
  flex-shrink: 0;
  border-right: 1px solid var(--border-color);
  padding: 8px;
  display: flex;
  flex-direction: column;
  gap: 4px;
  overflow-y: auto;
}
.md-list-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 8px;
  padding: 4px 8px;
  border-radius: 4px;
  cursor: pointer;
  font-size: var(--font-size-sm);
}
.md-list-item:hover {
  background-color: var(--hover-bg);
}
.md-list-item.active {
  background-color: var(--primary-color);
  color: white;
}
.md-detail {
  flex: 1;
  min-width: 0;
  padding: 12px;
  min-height: 0;
}
.md-list-item > span {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.md-list-item .n-button {
  flex-shrink: 0;
}
.md-field {
  margin-bottom: 8px;
}
</style>
