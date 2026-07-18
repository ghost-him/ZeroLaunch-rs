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
  userArgCount: number
  hasSystemParams: boolean
  triggerKeywords: string[]
}

export interface ResultAction {
  id: string
  label: string
  icon: string
  isDefault: boolean
  shortcutKey: string
}

export interface InlineParamData {
  candidateId: number
  triggerKeyword: string
  userArgCount: number
}

export type BridgeQueryResponse =
  | { mode: 'search'; results: ListItem[]; panelType?: never; panelData?: never; panelActions?: never; inlineParam?: never }
  | { mode: 'empty'; results: ListItem[]; panelType?: never; panelData?: never; panelActions?: never; inlineParam?: never }
  | { mode: 'plugin_panel'; results: ListItem[]; panelType: string; panelData: unknown; panelActions: ResultAction[]; inlineParam?: never }
  | { mode: 'plugin_immersive'; results: ListItem[]; panelType: string; panelData: unknown; panelActions: ResultAction[]; inlineParam?: never }
  | { mode: 'inline_param'; results: never[]; inlineParam: InlineParamData }

export interface ConfirmPayload {
  candidateId: number
  actionId: string
  queryText: string
  userArgs?: string[]
}

export type ConfirmResponse =
  | { status: 'executed' }
  | { status: 'enterParamPanel'; candidateId: number; userArgCount: number }

// ---- 配置相关 ----

export type ComponentType =
  | 'DataSource'
  | 'KeywordOptimizer'
  | 'KeywordInjector'
  | 'SearchEngine'
  | 'ScoreBooster'
  | 'ActionExecutor'
  | 'Plugin'
  | 'BiasRule'
  | 'Core'

export interface ComponentInfo {
  componentId: string
  componentName: string
  componentDescription: string
  componentType: ComponentType
  priority: number
  enabled: boolean
  defaultEnabled: boolean
}
export interface ComponentSchema {
  componentId: string
  componentName: string
  componentDescription: string
  componentType: ComponentType
  settings: SettingDefinition[]
}

export interface SettingDefinition {
  field: FieldDefinition
  group?: string
  order: number
  configAction?: string
  detailAction?: DetailActionDef
}

export interface DetailActionDef {
  action: string
  paramField: string
  paramKey: string
  previewItemKey: string
  previewItemLabel: string
  targetField: string
  targetMatchKey: string
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
  | 'text'
  | { number: { min?: number; max?: number; step?: number } }
  | 'boolean'
  | { select: { options: string[] } }
  | { path: { mode: 'file' | 'directory' } }
  | 'color'
  | 'json'
  | { array: { item: ArrayItem; minItems?: number; maxItems?: number; uiHint: ArrayUiHint } }
  | { image: { accept: string[]; maxSize?: number } }

export type ArrayItem =
  | { primitive: PrimitiveType }
  | { object: FieldDefinition[] }

export type PrimitiveType =
  | 'text'
  | { number: { min?: number; max?: number; step?: number } }
  | 'boolean'
  | { select: { options: string[] } }
  | { path: { mode: 'file' | 'directory' } }
  | 'color'

export type ArrayUiHint =
  | 'default'
  | 'table'
  | 'masterDetail'
  | 'tags'
  | { searchTable: { sourceComponent: string; sourceAction: string } }

export interface ConfigActionDef {
  action: string
  label: string
  description: string
}

// ---- 候选项摘要（用于 SearchTable 搜索结果） ----

export interface CandidateSummary {
  name: string
  target: string
  targetType: string
  icon: string
}

// ---- 事件载荷 ----

export interface ConfigChangedPayload {
  componentId: string
  componentType: ComponentType
}

export interface ConfigErrorPayload {
  componentId: string
  error: string
}

export interface InstallationEventPayload {
  eventType: 'install' | 'uninstall'
  appName: string
}

// ---- 插件键盘事件 ----

export interface PluginKeyEvent {
  key: string
  code: string
  ctrlKey: boolean
  shiftKey: boolean
  altKey: boolean
  metaKey: boolean
}

export interface PluginKeyEventResponse {
  handled: boolean
  exitPlugin: boolean
  panelUpdate: unknown | null
}

// ---- Plugin Inspector ----

export interface InspectorStateResponse {
  available?: boolean
  message?: string
  registeredPlugins?: PluginInspectorInfo[]
  recentQueries?: InspectedQueryEvent[]
  totalQueriesLogged?: number
}

export interface PluginInspectorInfo {
  componentId: string
  componentName: string
  componentType: string
  enabled: boolean
}

export interface InspectedQueryEvent {
  timestamp: string
  traceId: string
  rawQuery: string
  mode: string
  resultCount: number
  durationMs: number
}

// ---- Third-party Plugin Events ----

export interface PluginEventPayload {
  pluginId: string
  name?: string
  version?: string
}

// ---- Debug Tools ----

export interface SearchTimingResult {
  durationMs: number
  resultCount: number
  totalCandidates: number
}

export interface IndexTimingResult {
  durationMs: number
  candidateCount: number
}

export interface SearchDetailItem {
  rank: number
  candidateId: number
  name: string
  score: number
  targetType: string
  keywords: string[]
}
