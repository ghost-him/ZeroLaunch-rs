// ============================================================
// Schema 类型工具 — 类型守卫 + 配置提取器
// 所有 SettingType / ArrayItem 类型判断集中在此模块
// ============================================================

import type {
  SettingType,
  ArrayItem,
  ArrayUiHint,
  FieldDefinition,
  PrimitiveType,
} from '../bridge/contract'

// ---- SettingType 类型守卫 ----

export function isTextType(st: SettingType): st is 'text' {
  return st === 'text'
}

export function isNumberType(st: SettingType): st is { number: { min?: number; max?: number; step?: number } } {
  return typeof st === 'object' && st !== null && 'number' in st
}

export function isBooleanType(st: SettingType): st is 'boolean' {
  return st === 'boolean'
}

export function isSelectType(st: SettingType): st is { select: { options: string[] } } {
  return typeof st === 'object' && st !== null && 'select' in st
}

export function isPathType(st: SettingType): st is { path: { mode: 'file' | 'directory' } } {
  return typeof st === 'object' && st !== null && 'path' in st
}

export function isColorType(st: SettingType): st is 'color' {
  return st === 'color'
}

export function isJsonType(st: SettingType): st is 'json' {
  return st === 'json'
}

export function isArrayType(st: SettingType): st is {
  array: { item: ArrayItem; minItems?: number; maxItems?: number; uiHint: ArrayUiHint }
} {
  return typeof st === 'object' && st !== null && 'array' in st
}

export function isImageType(st: SettingType): st is { image: { accept: string[]; maxSize?: number } } {
  return typeof st === 'object' && st !== null && 'image' in st
}

// ---- SettingType 配置提取器 ----

export function getNumberConfig(st: SettingType) {
  if (isNumberType(st)) {
    return st.number
  }
  return { min: undefined, max: undefined, step: undefined } as {
    min?: number
    max?: number
    step?: number
  }
}

export function getSelectOptions(st: SettingType): { label: string; value: string }[] {
  if (isSelectType(st)) {
    return st.select.options.map((o) => ({ label: o, value: o }))
  }
  return []
}

export function getPathMode(st: SettingType): 'file' | 'directory' {
  if (isPathType(st)) {
    return st.path.mode
  }
  return 'file'
}

export function getArrayConfig(st: SettingType) {
  if (isArrayType(st)) {
    return st.array
  }
  return null
}

export function getImageConfig(st: SettingType): { accept: string[]; maxSize?: number } {
  if (isImageType(st)) {
    return st.image
  }
  return { accept: ['png', 'jpg', 'jpeg', 'webp'], maxSize: 2 * 1024 * 1024 }
}

// FieldDefinition-level helpers (for object array sub-fields)

export function isFieldNumber(fd: FieldDefinition): fd is FieldDefinition & {
  settingType: { number: { min?: number; max?: number; step?: number } }
} {
  return isNumberType(fd.settingType)
}

export function isFieldSelect(fd: FieldDefinition): fd is FieldDefinition & {
  settingType: { select: { options: string[] } }
} {
  return isSelectType(fd.settingType)
}

export function isFieldPath(fd: FieldDefinition): fd is FieldDefinition & {
  settingType: { path: { mode: 'file' | 'directory' } }
} {
  return isPathType(fd.settingType)
}

export function isFieldBoolean(fd: FieldDefinition): boolean {
  return fd.settingType === 'boolean'
}

// ---- ArrayItem 类型守卫 ----

export function isPrimitiveArray(item: ArrayItem): item is { primitive: PrimitiveType } {
  return typeof item === 'object' && item !== null && 'primitive' in item
}

export function isObjectArray(item: ArrayItem): item is { object: FieldDefinition[] } {
  return typeof item === 'object' && item !== null && 'object' in item
}

// ---- PrimitiveType 工具 ----

export type PrimitiveTypeName = 'text' | 'number' | 'boolean' | 'select' | 'path' | 'color'

export function getPrimitiveItemType(item: ArrayItem): PrimitiveTypeName {
  if (!isPrimitiveArray(item)) return 'text'
  const prim = item.primitive
  if (typeof prim === 'string') {
    if (prim === 'text' || prim === 'boolean' || prim === 'color') return prim
    return 'text'
  }
  if (typeof prim === 'object' && prim !== null) {
    if ('number' in prim) return 'number'
    if ('select' in prim) return 'select'
    if ('path' in prim) return 'path'
  }
  return 'text'
}

export function getPrimitiveNumberConfig(item: ArrayItem) {
  if (!isPrimitiveArray(item)) return {}
  const prim = item.primitive
  if (typeof prim === 'object' && prim !== null && 'number' in prim) {
    return prim.number
  }
  return {}
}

export function getPrimitiveSelectOptions(item: ArrayItem): { label: string; value: string }[] {
  if (!isPrimitiveArray(item)) return []
  const prim = item.primitive
  if (typeof prim === 'object' && prim !== null && 'select' in prim) {
    return prim.select.options.map((o) => ({ label: o, value: o }))
  }
  return []
}

export function getPrimitivePathMode(item: ArrayItem): 'file' | 'directory' {
  if (!isPrimitiveArray(item)) return 'file'
  const prim = item.primitive
  if (typeof prim === 'object' && prim !== null && 'path' in prim) {
    return prim.path.mode
  }
  return 'file'
}

// ---- Object array helpers ----

export function getVisibleObjectFields(item: ArrayItem): FieldDefinition[] {
  if (!isObjectArray(item)) return []
  return item.object.filter((f) => f.visible !== false)
}

// ---- 默认值生成 ----

export function getDefaultArrayItem(
  item: ArrayItem,
  defaultValue?: unknown,
): unknown {
  if (isPrimitiveArray(item)) {
    if (Array.isArray(defaultValue) && defaultValue.length > 0) return defaultValue[0]
    switch (getPrimitiveItemType(item)) {
      case 'number': return 0
      case 'boolean': return false
      default: return ''
    }
  }
  if (isObjectArray(item)) {
    const obj: Record<string, unknown> = {}
    for (const fd of getVisibleObjectFields(item)) {
      obj[fd.key] = fd.defaultValue ?? ''
    }
    return obj
  }
  return ''
}

// ---- ArrayUiHint 类型守卫 ----

export function isDefaultUiHint(hint: ArrayUiHint): hint is 'default' {
  return hint === 'default'
}

export function isTableUiHint(hint: ArrayUiHint): hint is 'table' {
  return hint === 'table'
}

export function isMasterDetailUiHint(hint: ArrayUiHint): hint is 'masterDetail' {
  return hint === 'masterDetail'
}

export function isTagsUiHint(hint: ArrayUiHint): hint is 'tags' {
  return hint === 'tags'
}

export function isSearchTableUiHint(
  hint: ArrayUiHint,
): hint is { searchTable: { sourceComponent: string; sourceAction: string; fieldMapping?: [string, string][] } } {
  return typeof hint === 'object' && hint !== null && 'searchTable' in hint
}

export function getSearchTableSource(
  hint: ArrayUiHint,
): { sourceComponent: string; sourceAction: string } | null {
  if (isSearchTableUiHint(hint)) {
    return hint.searchTable
  }
  return null
}

export function getSearchTableFieldMapping(
  hint: ArrayUiHint,
): [string, string][] | null {
  if (isSearchTableUiHint(hint)) {
    return hint.searchTable.fieldMapping ?? null
  }
  return null
}
