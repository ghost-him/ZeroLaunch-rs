// ============================================================
// ZeroLaunch 前后端共享类型定义
// 与 Rust 侧 serde(rename_all = "camelCase") 对齐
// ============================================================

// ---- 搜索 & 会话 ----

export interface ListItem {
  id: number
  title: string
  subtitle: string
  icon: string
  score: number
  actions: ResultAction[]
  targetType: string
}

export interface ResultAction {
  id: string
  label: string
  icon: string
  isDefault: boolean
  shortcutKey: string
}

export type BridgeQueryResponse =
  | { mode: 'search'; results: ListItem[]; panelType?: never; panelData?: never; panelActions?: never }
  | { mode: 'empty'; results: ListItem[]; panelType?: never; panelData?: never; panelActions?: never }
  | { mode: 'plugin_panel'; results: ListItem[]; panelType: string; panelData: unknown; panelActions: ResultAction[] }
  | { mode: 'plugin_immersive'; results: ListItem[]; panelType: string; panelData: unknown; panelActions: ResultAction[] }

export interface ConfirmPayload {
  candidateId: number
  actionId: string
  queryText: string
  userArgs?: string[]
}

// ---- 配置相关 ----

export type ComponentType =
  | 'DataSource'
  | 'KeywordOptimizer'
  | 'SearchEngine'
  | 'ScoreBooster'
  | 'ActionExecutor'
  | 'Plugin'
  | 'Core'

export interface ComponentInfo {
  componentId: string
  componentName: string
  componentType: ComponentType
  enabled: boolean
  defaultEnabled: boolean
}

export interface ComponentSchema {
  componentId: string
  componentName: string
  componentType: ComponentType
  settings: SettingDefinition[]
}

export interface SettingDefinition {
  field: FieldDefinition
  group?: string
  order: number
  configAction?: string
}

export interface FieldDefinition {
  key: string
  label: string
  description: string
  settingType: SettingType
  defaultValue: unknown
  visible: boolean
  editable: boolean
}

export type SettingType =
  | 'Text'
  | { Number: { min?: number; max?: number; step?: number } }
  | 'Boolean'
  | { Select: { options: string[] } }
  | { Path: { mode: 'File' | 'Directory' } }
  | 'Color'
  | 'Json'
  | { Array: { item: ArrayItem; minItems?: number; maxItems?: number; uiHint: ArrayUiHint } }

export type ArrayItem =
  | { Primitive: PrimitiveType }
  | { Object: FieldDefinition[] }

export type PrimitiveType =
  | 'Text'
  | { Number: { min?: number; max?: number; step?: number } }
  | 'Boolean'
  | { Select: { options: string[] } }
  | { Path: { mode: 'File' | 'Directory' } }
  | 'Color'

export type ArrayUiHint = 'Default' | 'Table' | 'MasterDetail' | 'Tags'

export interface ConfigActionDef {
  action: string
  label: string
  description: string
}

// ---- 事件载荷 ----

export interface ConfigChangedPayload {
  componentId: string
  componentType: string
}

export interface ConfigErrorPayload {
  componentId: string
  error: string
}

export interface InstallationEventPayload {
  eventType: 'install' | 'uninstall'
  appName: string
}
