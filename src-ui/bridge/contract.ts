// ============================================================
// ZeroLaunch 前后端共享类型定义
// 与 Rust 侧 serde 对齐
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

export type PanelSubmitBehavior = 'execute' | 'requery'

export interface PanelInteraction {
  submitBehavior: PanelSubmitBehavior
  queryDebounceMs: number
}

export type BridgeQueryResponse =
  | { mode: 'search'; results: ListItem[]; panelType: null; panelData: null; panelActions: null; inlineParam: null; panelInteraction: null }
  | { mode: 'empty'; results: ListItem[]; panelType: null; panelData: null; panelActions: null; inlineParam: null; panelInteraction: null }
  | { mode: 'plugin_panel'; results: ListItem[]; panelType: string; panelData: unknown; panelActions: ResultAction[]; inlineParam: null; panelInteraction: PanelInteraction }
  | { mode: 'plugin_immersive'; results: ListItem[]; panelType: string; panelData: unknown; panelActions: ResultAction[]; inlineParam: null; panelInteraction: PanelInteraction }
  | { mode: 'inline_param'; results: never[]; panelType: null; panelData: null; panelActions: null; inlineParam: InlineParamData; panelInteraction: null }

export interface ConfirmPayload {
  candidateId: number
  actionId: string
  queryText: string
  userArgs?: string[]
}

export type ConfirmResponse =
  | { status: 'executed' }
  | { status: 'enterParamPanel'; candidateId: number; userArgCount: number }

// ---- 配置相关新类型（SchemaKind 驱动） ----

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


// ── Schema 类型系统 ──

export type SchemaNode =
  | { type: 'string'; enum: string[]; enumLabels: string[]; minLength: number | null; maxLength: number | null; pattern: string | null; default: unknown | null }
  | { type: 'number'; minimum: number | null; maximum: number | null; multipleOf: number | null; default: unknown | null }
  | { type: 'integer'; minimum: number | null; maximum: number | null; multipleOf: number | null; default: unknown | null }
  | { type: 'boolean'; default: unknown | null }
  | { type: 'array'; items: SchemaNode; itemWidget: WidgetHint | null; minItems: number | null; maxItems: number | null; default: unknown | null }
  | { type: 'object'; properties: Record<string, SchemaNode>; ui: FieldUiMetadata[]; required: string[]; default: unknown | null }

// ── UI 控件提示 ──

export type WidgetHint =
  | { kind: 'text' }
  | { kind: 'textarea' }
  | { kind: 'number' }
  | { kind: 'toggle' }
  | { kind: 'select' }
  | { kind: 'path'; mode: 'file' | 'directory' }
  | { kind: 'color' }
  | { kind: 'image'; accept: string[]; maxSize: number | null }
  | { kind: 'list' }
  | { kind: 'tags' }
  | { kind: 'table' }
  | { kind: 'cards' }
  | { kind: 'masterDetail' }
  | { kind: 'searchTable' }

// ── 数据注入 Action 绑定 ──

export interface DataActionBinding {
  action: string
  component: string | null
  labelField: string
  valueField: string
  fieldMapping: [string, string][]
}

export interface EffectActionBinding {
  action: string
  component: string | null
  fieldMapping: [string, string][]
  transient: boolean
}

export type FieldAction =
  | { kind: 'data'; binding: DataActionBinding }
  | { kind: 'effect'; binding: EffectActionBinding }

// ── 详情面板联动动作 ──

export interface DetailActionDef {
  action: string
  paramField: string
  paramKey: string
  previewItemKey: string
  previewItemLabel: string
  targetField: string
  targetMatchKey: string
}

// ── 字段 UI 元数据 ──

export interface FieldUiMetadata {
  pointer: string
  label: string
  description: string
  group: string | null
  order: number
  visible: boolean
  readOnly: boolean
  widget: WidgetHint | null
  action: FieldAction | null
  detailAction: DetailActionDef | null
}


// ── 配置贡献 ──

export interface SettingsContribution {
  schemaVersion: number
  properties: Record<string, SchemaNode>
  ui: FieldUiMetadata[]
  commitPolicy: 'staged' | 'immediateAllowed'
}


/** 组件 Schema — IPC `config_get_schema` 的返回值包装。 */
export interface ComponentSchema {
  componentId: string
  componentName: string
  componentDescription: string
  componentType: ComponentType
  contribution: SettingsContribution
}

// ── 配置动作 ──

export interface ConfigActionDef {
  action: string
  label: string
  description: string
}

export interface ConfigActionPayload {
  componentId: string
  action: string
  params?: unknown
}

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
