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

export function isTextType(st: SettingType): st is 'Text' {
  return st === 'Text'
}

export function isNumberType(st: SettingType): st is { Number: { min?: number; max?: number; step?: number } } {
  return typeof st === 'object' && st !== null && 'Number' in st
}

export function isBooleanType(st: SettingType): st is 'Boolean' {
  return st === 'Boolean'
}

export function isSelectType(st: SettingType): st is { Select: { options: string[] } } {
  return typeof st === 'object' && st !== null && 'Select' in st
}

export function isPathType(st: SettingType): st is { Path: { mode: 'File' | 'Directory' } } {
  return typeof st === 'object' && st !== null && 'Path' in st
}

export function isColorType(st: SettingType): st is 'Color' {
  return st === 'Color'
}

export function isJsonType(st: SettingType): st is 'Json' {
  return st === 'Json'
}

export function isArrayType(st: SettingType): st is {
  Array: { item: ArrayItem; minItems?: number; maxItems?: number; uiHint: ArrayUiHint }
} {
  return typeof st === 'object' && st !== null && 'Array' in st
}

// ---- SettingType 配置提取器 ----

export function getNumberConfig(st: SettingType) {
  if (isNumberType(st)) {
    return st.Number
  }
  return { min: undefined, max: undefined, step: undefined } as {
    min?: number
    max?: number
    step?: number
  }
}

export function getSelectOptions(st: SettingType): { label: string; value: string }[] {
  if (isSelectType(st)) {
    return st.Select.options.map((o) => ({ label: o, value: o }))
  }
  return []
}

export function getPathMode(st: SettingType): 'File' | 'Directory' {
  if (isPathType(st)) {
    return st.Path.mode
  }
  return 'File'
}

export function getArrayConfig(st: SettingType) {
  if (isArrayType(st)) {
    return st.Array
  }
  return null
}

// FieldDefinition-level helpers (for object array sub-fields)

export function isFieldNumber(fd: FieldDefinition): fd is FieldDefinition & {
  settingType: { Number: { min?: number; max?: number; step?: number } }
} {
  return isNumberType(fd.settingType)
}

export function isFieldSelect(fd: FieldDefinition): fd is FieldDefinition & {
  settingType: { Select: { options: string[] } }
} {
  return isSelectType(fd.settingType)
}

export function isFieldPath(fd: FieldDefinition): fd is FieldDefinition & {
  settingType: { Path: { mode: 'File' | 'Directory' } }
} {
  return isPathType(fd.settingType)
}

export function isFieldBoolean(fd: FieldDefinition): boolean {
  return fd.settingType === 'Boolean'
}

// ---- ArrayItem 类型守卫 ----

export function isPrimitiveArray(item: ArrayItem): item is { Primitive: PrimitiveType } {
  return typeof item === 'object' && item !== null && 'Primitive' in item
}

export function isObjectArray(item: ArrayItem): item is { Object: FieldDefinition[] } {
  return typeof item === 'object' && item !== null && 'Object' in item
}

// ---- PrimitiveType 工具 ----

export type PrimitiveTypeName = 'Text' | 'Number' | 'Boolean' | 'Select' | 'Path' | 'Color'

export function getPrimitiveItemType(item: ArrayItem): PrimitiveTypeName {
  if (!isPrimitiveArray(item)) return 'Text'
  const prim = item.Primitive
  if (typeof prim === 'string') {
    if (prim === 'Text' || prim === 'Boolean' || prim === 'Color') return prim
    return 'Text'
  }
  if (typeof prim === 'object' && prim !== null) {
    if ('Number' in prim) return 'Number'
    if ('Select' in prim) return 'Select'
    if ('Path' in prim) return 'Path'
  }
  return 'Text'
}

export function getPrimitiveNumberConfig(item: ArrayItem) {
  if (!isPrimitiveArray(item)) return {}
  const prim = item.Primitive
  if (typeof prim === 'object' && prim !== null && 'Number' in prim) {
    return prim.Number
  }
  return {}
}

export function getPrimitiveSelectOptions(item: ArrayItem): { label: string; value: string }[] {
  if (!isPrimitiveArray(item)) return []
  const prim = item.Primitive
  if (typeof prim === 'object' && prim !== null && 'Select' in prim) {
    return prim.Select.options.map((o) => ({ label: o, value: o }))
  }
  return []
}

export function getPrimitivePathMode(item: ArrayItem): 'File' | 'Directory' {
  if (!isPrimitiveArray(item)) return 'File'
  const prim = item.Primitive
  if (typeof prim === 'object' && prim !== null && 'Path' in prim) {
    return prim.Path.mode
  }
  return 'File'
}

// ---- Object array helpers ----

export function getVisibleObjectFields(item: ArrayItem): FieldDefinition[] {
  if (!isObjectArray(item)) return []
  return item.Object.filter((f) => f.visible !== false)
}

// ---- 默认值生成 ----

export function getDefaultArrayItem(
  item: ArrayItem,
  defaultValue?: unknown,
): unknown {
  if (isPrimitiveArray(item)) {
    if (Array.isArray(defaultValue) && defaultValue.length > 0) return defaultValue[0]
    switch (getPrimitiveItemType(item)) {
      case 'Number': return 0
      case 'Boolean': return false
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
