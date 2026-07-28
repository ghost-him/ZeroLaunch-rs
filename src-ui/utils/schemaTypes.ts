// ============================================================
// Schema 类型工具 — 字段配置合并 + 工具函数
// ============================================================

import type {
  SchemaNode,
  WidgetHint,
  FieldAction,
  DetailActionDef,
  SettingsContribution,
} from '../bridge/contract'

// ── 字段配置（字段组件消费的完整类型） ──

/** 当前前端可渲染的 SettingsContribution schema 版本。 */
export const SUPPORTED_SCHEMA_VERSION = 1
/** 字段组件接收的合并配置。由 SettingsContribution 的 properties + ui 合并而来。 */
export interface FieldConfig {
  key: string
  label: string
  description: string
  group: string | null
  order: number
  visible: boolean
  required: boolean
  readOnly: boolean
  schema: SchemaNode
  widget: WidgetHint | null
  action: FieldAction | null
  detailAction: DetailActionDef | null
}

/** 数组或对象子项字段定义，完整保留嵌套 schema 的 UI 元数据。 */
export interface FieldDef {
  key: string
  label: string
  description: string
  group: string | null
  order: number
  visible: boolean
  required: boolean
  schema: SchemaNode
  widget: WidgetHint | null
  editable: boolean
  action: FieldAction | null
  detailAction: DetailActionDef | null
}

// ── 构建函数 ──

/** 从 SettingsContribution 构建字段配置列表，并验证 UI metadata 完整覆盖 properties。 */
export function buildFieldConfigs(contribution: SettingsContribution): FieldConfig[] {
  const { properties, ui } = contribution
  validateSchemaMetadata(contribution)

  if (ui.length !== Object.keys(properties).length) {
    throw new Error('设置 schema 的 UI metadata 必须与 properties 一一对应')
  }
  const seen = new Set<string>()
  return ui
    .map((metadata) => {
      const key = pointerToKey(metadata.pointer)
      if (!seen.add(key)) throw new Error(`设置 schema 存在重复 UI 指针: ${metadata.pointer}`)
      const schema = properties[key]
      if (!schema) {
        throw new Error(`设置 schema 缺少 UI 指针对应字段: ${metadata.pointer}`)
      }
      return {
        key,
        label: metadata.label,
        description: metadata.description,
        group: metadata.group ?? null,
        order: metadata.order,
        visible: metadata.visible,
        required: false,
        readOnly: metadata.readOnly,
        schema,
        widget: metadata.widget ?? null,
        action: metadata.action ?? null,
        detailAction: metadata.detailAction ?? null,
      }
    })
    .filter((field) => field.visible)
    .sort((a, b) => a.order - b.order)
}

/** 删除 schema 声明为 transient 的 effect 字段，保留其他值供后端最终校验。 */
export function stripTransientSettings(
  contribution: SettingsContribution,
  settings: Record<string, unknown>,
): Record<string, unknown> {
  return stripTransientValue(
    { type: 'object', properties: contribution.properties, ui: contribution.ui, required: [], default: null },
    settings,
  ) as Record<string, unknown>
}

/** 递归过滤 object/array 中的 transient effect 字段。 */
function stripTransientValue(
  schema: SchemaNode,
  value: unknown,
): unknown {
  if (schema.type === 'array') {
    return Array.isArray(value) ? value.map((item) => stripTransientValue(schema.items, item)) : value
  }
  if (schema.type !== 'object' || value === null || typeof value !== 'object' || Array.isArray(value)) {
    return value
  }
  const result = { ...(value as Record<string, unknown>) }
  for (const [key, childSchema] of Object.entries(schema.properties)) {
    const metadata = schema.ui.find((item) => pointerToKey(item.pointer) === key)
    if (metadata?.action?.kind === 'effect' && metadata.action.binding.transient) {
      delete result[key]
      continue
    }
    if (key in result) result[key] = stripTransientValue(childSchema, result[key])
  }
  return result
}

// ── 工具函数 ──

/** 将单段 JSON Pointer（如 "/theme"、"/a~1b"）解码为字段 key。 */
export function pointerToKey(pointer: string): string {
  if (!pointer.startsWith('/')) throw new Error(`无效的 JSON Pointer: ${pointer}`)
  const encoded = pointer.slice(1)
  if (encoded.includes('/') || /~(?![01])/.test(encoded)) {
    throw new Error(`无效的 JSON Pointer: ${pointer}`)
  }
  return encoded.replace(/~1/g, '/').replace(/~0/g, '~')
}

/** 递归检查完整 schema 的 UI metadata，确保错误在字段渲染前可见。 */
function validateSchemaMetadata(contribution: SettingsContribution): void {
  for (const [key, schema] of Object.entries(contribution.properties)) {
    validateSchemaNodeMetadata(schema, `/${key}`)
  }
}

/** 递归检查 object/array 节点的 UI metadata 与 schema 结构。 */
function validateSchemaNodeMetadata(schema: SchemaNode, path: string): void {
  if (schema.type === 'array') {
    validateSchemaNodeMetadata(schema.items, `${path}/items`)
    return
  }
  if (schema.type !== 'object') return
  if (schema.ui.length !== Object.keys(schema.properties).length) {
    throw new Error(`${path} 的 UI metadata 必须与 properties 一一对应`)
  }
  const seen = new Set<string>()
  for (const metadata of schema.ui) {
    const key = pointerToKey(metadata.pointer)
    if (!seen.add(key)) throw new Error(`${path} 存在重复 UI 指针: ${metadata.pointer}`)
    const childSchema = schema.properties[key]
    if (!childSchema) throw new Error(`${path} 的 UI 指针没有对应属性: ${metadata.pointer}`)
    validateSchemaNodeMetadata(childSchema, `${path}/${key}`)
  }
}

// ── 数组 UI 类型 ──


/** 数组 widget 的类型标识（从 WidgetHint 映射而来）。 */
export type ArrayUiKind = 'list' | 'table' | 'tags' | 'cards' | 'masterDetail' | 'searchTable'

/** 将 WidgetHint 映射为数组 UI 类型标识。 */
export function widgetToArrayUiKind(widget: WidgetHint | null): ArrayUiKind {
  switch (widget?.kind) {
    case 'table': return 'table'
    case 'tags': return 'tags'
    case 'cards': return 'cards'
    case 'masterDetail': return 'masterDetail'
    case 'searchTable': return 'searchTable'
    default: return 'list'
  }
}

/** 字段渲染器类别，供 DynamicFormField 统一分派。 */
export type FieldRendererKind = 'text' | 'number' | 'boolean' | 'select' | 'color' | 'path' | 'image' | 'array' | 'object'

/** 字段 schema/widget 组合的基础分派结果。 */
export interface FieldRenderInfo {
  kind: FieldRendererKind | null
  schemaType: string
  widgetKind: string | null
  error: 'unsupportedWidget' | 'unknownSchema' | null
}

/** 数组元素支持的基础 schema 类型。 */
export type ArrayItemKind = 'string' | 'number' | 'integer' | 'boolean' | 'object' | 'unsupported'

/** 将数组元素 schema 映射为渲染层使用的基础类型标识。 */
export function getArrayItemKind(schema: SchemaNode | null): ArrayItemKind | null {
  if (!schema) return null
  switch (schema.type) {
    case 'string': return 'string'
    case 'number': return 'number'
    case 'integer': return 'integer'
    case 'boolean': return 'boolean'
    case 'object': return 'object'
    default: return 'unsupported'
  }
}

/** 获取 schema 类型名称，供不支持的组合显示诊断信息。 */
export function getSchemaTypeName(schema: SchemaNode | null): string {
  return schema ? String(schema.type) : 'unknown'
}

/** 支持的数组 widget 静态集合。 */
const ARRAY_WIDGET_KINDS: Record<string, true> = {
  list: true,
  tags: true,
  table: true,
  cards: true,
  masterDetail: true,
  searchTable: true,
}

/** 校验 schema/widget 组合并返回字段渲染器类别。 */
export function getFieldRenderInfo(field: FieldConfig): FieldRenderInfo {
  const { schema, widget } = field
  const widgetKind = widget?.kind ?? null
  switch (schema.type) {
    case 'string':
      if (!widget) return { kind: schema.enum && schema.enum.length > 0 ? 'select' : 'text', schemaType: schema.type, widgetKind, error: null }
      if (widget.kind === 'select') return { kind: 'select', schemaType: schema.type, widgetKind, error: null }
      if (widget.kind === 'color') return { kind: 'color', schemaType: schema.type, widgetKind, error: null }
      if (widget.kind === 'path') return { kind: 'path', schemaType: schema.type, widgetKind, error: null }
      if (widget.kind === 'image') return { kind: 'image', schemaType: schema.type, widgetKind, error: null }
      if (widget.kind === 'text' || widget.kind === 'textarea') return { kind: 'text', schemaType: schema.type, widgetKind, error: null }
      return { kind: null, schemaType: schema.type, widgetKind, error: 'unsupportedWidget' }
    case 'number':
    case 'integer':
      return !widget || widget.kind === 'number'
        ? { kind: 'number', schemaType: schema.type, widgetKind, error: null }
        : { kind: null, schemaType: schema.type, widgetKind, error: 'unsupportedWidget' }
    case 'boolean':
      return !widget || widget.kind === 'toggle'
        ? { kind: 'boolean', schemaType: schema.type, widgetKind, error: null }
        : { kind: null, schemaType: schema.type, widgetKind, error: 'unsupportedWidget' }
    case 'array':
      return !widget || ARRAY_WIDGET_KINDS[widget.kind] === true
        ? { kind: 'array', schemaType: schema.type, widgetKind, error: null }
        : { kind: null, schemaType: schema.type, widgetKind, error: 'unsupportedWidget' }
    case 'object':
      return !widget
        ? { kind: 'object', schemaType: schema.type, widgetKind, error: null }
        : { kind: null, schemaType: schema.type, widgetKind, error: 'unsupportedWidget' }
    default:
      return { kind: null, schemaType: String(schema), widgetKind, error: 'unknownSchema' }
  }
}

/** 判断并收窄为 object schema。 */
export function isObjectSchema(schema: SchemaNode): schema is Extract<SchemaNode, { type: 'object' }> {
  return schema.type === 'object'
}
export type PrimitiveArrayEditorKind = 'text' | 'select' | 'number' | 'boolean' | 'path' | 'color'

/** 获取数组元素 schema；非数组节点返回 null。 */
export function getArrayItemSchema(schema: SchemaNode): SchemaNode | null {
  return schema.type === 'array' ? schema.items : null
}

/** 数组数量约束，供不同数组控件统一消费。 */
export interface ArrayConstraints {
  minItems: number | null
  maxItems: number | null
}

/** 读取数组 schema 的 minItems/maxItems。 */
export function getArrayConstraints(schema: SchemaNode): ArrayConstraints {
  return schema.type === 'array'
    ? { minItems: schema.minItems, maxItems: schema.maxItems }
    : { minItems: null, maxItems: null }
}

/** 构造数组原语编辑器使用的字段配置，不生成业务默认值。 */
export function getArrayItemFieldConfig(field: FieldConfig): FieldConfig | null {
  const itemSchema = getArrayItemSchema(field.schema)
  if (!itemSchema || itemSchema.type === 'object') return null
  const itemWidget = field.schema.type === 'array'
    ? field.schema.itemWidget ?? defaultWidgetForSchema(itemSchema)
    : defaultWidgetForSchema(itemSchema)
  return {
    ...field,
    key: `${field.key}[]`,
    schema: itemSchema,
    widget: itemWidget,
    action: null,
    detailAction: null,
  }
}

/** 根据 itemWidget 选择并校验原语数组编辑器。 */
export function getPrimitiveArrayEditorKind(field: FieldConfig): PrimitiveArrayEditorKind | null {
  const itemField = getArrayItemFieldConfig(field)
  if (!itemField) return null
  switch (itemField.widget?.kind) {
    case 'path': return itemField.schema.type === 'string' ? 'path' : null
    case 'color': return itemField.schema.type === 'string' ? 'color' : null
    case 'select': return itemField.schema.type === 'string' ? 'select' : null
    case 'text':
    case 'textarea': return itemField.schema.type === 'string' ? 'text' : null
    case 'number': return itemField.schema.type === 'number' || itemField.schema.type === 'integer' ? 'number' : null
    case 'toggle': return itemField.schema.type === 'boolean' ? 'boolean' : null
    default: return null
  }
}


/** 为没有显式 itemWidget 的原语 schema 提供基础编辑器提示。 */
function defaultWidgetForSchema(schema: SchemaNode): WidgetHint | null {
  switch (schema.type) {
    case 'string': return schema.enum && schema.enum.length > 0 ? { kind: 'select' } : { kind: 'text' }
    case 'number':
    case 'integer': return { kind: 'number' }
    case 'boolean': return { kind: 'toggle' }
    default: return null
  }
}

/** 从 SchemaNode 中获取数值范围、步长和整数精度。 */
export function getSchemaNumberConfig(schema: SchemaNode): { min: number | null; max: number | null; step: number | null; precision?: number } {
  if (schema.type === 'number') {
    return { min: schema.minimum, max: schema.maximum, step: schema.multipleOf }
  }
  if (schema.type === 'integer') {
    return { min: schema.minimum, max: schema.maximum, step: schema.multipleOf, precision: 0 }
  }
  return { min: null, max: null, step: null }
}
/** 从 SchemaNode 中获取字符串长度和 pattern 约束。 */
export function getSchemaStringConfig(schema: SchemaNode): { minLength: number | null; maxLength: number | null; pattern: string | null } {
  if (schema.type === 'string') {
    return { minLength: schema.minLength, maxLength: schema.maxLength, pattern: schema.pattern }
  }
  return { minLength: null, maxLength: null, pattern: null }
}

/** 从 SchemaNode 中获取下拉选项。 */
export function getSchemaEnumOptions(schema: SchemaNode): { label: string; value: string }[] {
  if (schema.type === 'string' && schema.enum) {
    return schema.enum.map((v) => ({ label: v, value: v }))
  }
  return []
}

/** 从 SchemaNode 中获取路径模式。 */
export function getSchemaPathMode(widget: WidgetHint | null): 'file' | 'directory' {
  if (widget?.kind === 'path') return widget.mode
  return 'file'
}

/** 获取文本字段的输入模式。 */
export function getTextInputKind(widget: WidgetHint | null): 'text' | 'textarea' {
  return widget?.kind === 'textarea' ? 'textarea' : 'text'
}

/** 从 SchemaNode 中获取图片配置。 */
export function getSchemaImageConfig(widget: WidgetHint | null): { accept: string[]; maxSize: number | null } {
  if (widget?.kind === 'image') return { accept: widget.accept, maxSize: widget.maxSize }
  return { accept: ['png', 'jpg', 'jpeg', 'webp', 'ico'], maxSize: 2 * 1024 * 1024 }
}
/** 前端仅做基础结构校验，后端 ConfigManager 是最终 schema 校验权威。 */
export function validateSettings(
  contribution: SettingsContribution,
  settings: unknown,
): string | null {
  if (settings === null || typeof settings !== 'object' || Array.isArray(settings)) {
    return '$ must be an object'
  }
  const value = settings as Record<string, unknown>
  for (const key of Object.keys(value)) {
    if (!(key in contribution.properties)) return `/${key} is not defined by schema`
  }
  for (const [key, schema] of Object.entries(contribution.properties)) {
    const error = validateSchemaValue(schema, value[key], `/${key}`)
    if (error) return error
  }
  return null
}

/** 只检查值的基础 JSON 结构，不在前端复制后端约束规则。 */
function validateSchemaValue(schema: SchemaNode, value: unknown, path: string): string | null {
  if (value === undefined) return null
  switch (schema.type) {
    case 'string':
      return typeof value === 'string' ? null : `${path} must be a string`
    case 'number':
      return typeof value === 'number' && Number.isFinite(value) ? null : `${path} must be a finite number`
    case 'integer':
      return typeof value === 'number' && Number.isInteger(value) ? null : `${path} must be an integer`
    case 'boolean':
      return typeof value === 'boolean' ? null : `${path} must be a boolean`
    case 'array':
      if (!Array.isArray(value)) return `${path} must be an array`
      for (const [index, item] of value.entries()) {
        const error = validateSchemaValue(schema.items, item, `${path}/${index}`)
        if (error) return error
      }
      return null
    case 'object': {
      if (value === null || typeof value !== 'object' || Array.isArray(value)) {
        return `${path} must be an object`
      }
      for (const key of Object.keys(value)) {
        if (!(key in schema.properties)) return `${path}/${key} is not defined by schema`
      }
      for (const [key, child] of Object.entries(schema.properties)) {
        const error = validateSchemaValue(child, (value as Record<string, unknown>)[key], `${path}/${key}`)
        if (error) return error
      }
      return null
    }
    default:
      return null
  }
}

/** 从 object schema 的完整 UI metadata 构建字段列表，可选择保留隐藏字段。 */
export function getObjectFieldDefs(schema: SchemaNode, includeHidden = false): FieldDef[] {
  if (schema.type !== 'object') return []
  if (schema.ui.length !== Object.keys(schema.properties).length) {
    throw new Error('object schema 的 UI metadata 必须与 properties 一一对应')
  }
  const seen = new Set<string>()
  return [...schema.ui]
    .sort((a, b) => a.order - b.order)
    .map((ui) => {
      const key = pointerToKey(ui.pointer)
      if (!seen.add(key)) {
        throw new Error(`object schema 存在重复 UI 指针: ${ui.pointer}`)
      }
      const childSchema = schema.properties[key]
      if (!childSchema) {
        throw new Error(`object schema 的 UI 指针没有对应属性: ${ui.pointer}`)
      }
      return {
        key,
        label: ui.label,
        description: ui.description,
        group: ui.group ?? null,
        order: ui.order,
        visible: ui.visible,
        required: schema.required?.includes(key) ?? false,
        schema: childSchema,
        widget: ui.widget ?? null,
        editable: !ui.readOnly,
        action: ui.action ?? null,
        detailAction: ui.detailAction ?? null,
      }
    })
    .filter((field) => includeHidden || field.visible)
}

/** 判断数组当前是否允许继续添加元素。 */
export function canAddArrayItem(schema: SchemaNode, length: number): boolean {
  return schema.type !== 'array' || schema.maxItems === null || length < schema.maxItems
}

/** 判断数组当前是否允许继续删除元素。 */
export function canRemoveArrayItem(schema: SchemaNode, length: number): boolean {
  return schema.type !== 'array' || schema.minItems === null || length > schema.minItems
}

/** 生成数组项：优先使用后端明确 default，仅为 object/array 提供空结构容器。 */
export function getDefaultArrayItem(schema: SchemaNode): unknown {
  if (schema.default != null) return schema.default
  if (schema.type === 'object') return {}
  if (schema.type === 'array') return []
  return undefined
}

/** 将嵌套字段定义转换为可复用的 FieldConfig。 */
export function fieldDefToConfig(fd: FieldDef, parentReadOnly = false): FieldConfig {
  return {
    key: fd.key,
    label: fd.label,
    description: fd.description,
    group: fd.group,
    order: fd.order,
    visible: fd.visible,
    required: fd.required,
    readOnly: parentReadOnly || !fd.editable,
    schema: fd.schema,
    widget: fd.widget,
    action: fd.action,
    detailAction: fd.detailAction,
  }
}
