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

export type PanelQueryTrigger = 'onInput' | 'onEnter'

/** 面板按键动作 —— 宿主解释执行的动作语义（与 Rust PanelKeyAction 对齐）。 */
export type PanelKeyAction =
  | { kind: 'confirm'; /** Enter 标准语义：有可执行动作→执行默认动作；无动作→确认查询（宿主 confirmQuery 三分支）。 */ }
  | { kind: 'executeAction'; /** null = 执行面板默认动作；字符串 = 执行指定动作。 */ actionId: string | null }
  | { kind: 'goBack' }
  | { kind: 'gotoPanel'; panelId: string }
  | { kind: 'custom'; action: string; args: unknown }

export interface PanelKeyBinding {
  /** 按键格式："Enter" | "Ctrl+Enter" | "Escape" | "Tab" | "a"。 */
  key: string
  action: PanelKeyAction
}

export interface PanelInteraction {
  queryTrigger: PanelQueryTrigger
  queryDebounceMs: number
  /** 面板按键绑定列表 —— 声明即接管：命中绑定由宿主解释执行，未声明的键交还浏览器/输入框（宿主不做兜底）。 */
  bindings: PanelKeyBinding[]
}

/** 会话展示形态（session-state 事件 presentation 字段，camelCase 序列化，与后端 PresentationMode 对齐）。 */
export type PresentationMode =
  | 'none'
  | 'search'
  | 'inlineParam'
  | 'paramPanel'
  | 'pluginPanel'
  | 'pluginImmersive'

/** 会话状态事件 payload —— 整个会话系统的唯一事件（后端 Dispatcher 推送）。 */
export interface SessionStateEvent {
  /** 会话代际：归属/形态变化时递增（前端单调递增更新，随 confirm 回传校验）。 */
  generation: number
  presentation: PresentationMode
  /** 插件面板信息：对象 = 插件的面板元数据（pluginId 即会话归属）；null = 宿主面板（默认搜索归属）。 */
  panel: { pluginId: string; panelId: string } | null
  /** 插件面板交互契约（含按键映射）：对象 = 插件的按键声明；null = 宿主面板（default-search 归属），由宿主提供默认键。 */
  interaction: PanelInteraction | null
  /** 插件触发词列表，供「输入是否仍属于当前面板」的 IPC 前判定（镜像参数唯一来源）。 */
  triggerKeywords: string[]
}

export type BridgeQueryResponse =
  | { mode: 'search'; generation: number; results: ListItem[]; panelType: null; panelData: null; panelActions: null; inlineParam: null }
  | { mode: 'plugin_panel'; generation: number; results: ListItem[]; panelType: string; panelData: unknown; panelActions: ResultAction[]; inlineParam: null }
  | { mode: 'plugin_immersive'; generation: number; results: ListItem[]; panelType: string; panelData: unknown; panelActions: ResultAction[]; inlineParam: null }
  | { mode: 'inline_param'; generation: number; results: never[]; panelType: null; panelData: null; panelActions: null; inlineParam: InlineParamData }

/**
 * 确认请求 —— `bridge_confirm` 的 IPC 载荷（与后端 ConfirmRequestPayload tagged union 对齐）：
 * - `candidate`：宿主候选确认（默认搜索执行 / 插件面板默认动作）；
 * - `pluginAction`：插件面板动作（面板按键契约 Custom / GotoPanel 回插件）。
 */
export type ConfirmRequest =
  | {
      kind: 'candidate'
      candidateId: number
      actionId: string
      queryText: string
      userArgs?: string[]
      /** 会话代际：最后一次观测到的代际，后端据此拒绝过期确认（必填）。 */
      generation: number
    }
  | {
      kind: 'pluginAction'
      pluginId: string
      action: string
      args: unknown
      /** 会话代际：最后一次观测到的代际，后端据此拒绝过期面板动作（必填）。 */
      generation: number
    }

export type ConfirmResponse =
  | { status: 'executed'; generation: number }
  | {
      status: 'enterParamPanel'
      candidateId: number
      userArgCount: number
      generation: number
    }

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
  | { kind: 'font'; action: string; component: string | null }
  | { kind: 'hotkey' }
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
  /** labelField 列的表头显示文本；labelField 在条目 schema 中无对应字段时使用。 */
  labelFieldLabel: string
  valueField: string
  /** 字段级 data action：返回数组结果与当前字段值合并时的去重键；None 时整体替换。 */
  mergeKey: string | null
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

/** 安装监控事件 —— 与后端 InstallationEvent 对齐：一次去抖合并后的文件系统变化。 */
export interface InstallationEventPayload {
  /** 变化类型，与后端 InstallationEventKind 一一对应 */
  kind: 'created' | 'modified' | 'removed' | 'other'
  /** 发生变化的路径列表（可能为空） */
  changedPaths: string[]
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
  /** 后端会话归属标识（"default-search" | pluginId）。 */
  ownerId: string
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
  targetText: string
  keywords: string[]
  detailedScore: ScoreDetail[]
}

/** 单条分数分解明细：引擎或 ScoreBooster 计算的一项分数及其权重。 */
export interface ScoreDetail {
  /** 该项的原始分数（未乘权重）。 */
  score: number
  /** 该项的权重系数（add 项与 score 相乘后计入总分；multiply 项为乘到累计分上的系数）。 */
  weight: number
  /** 该项分数来源描述（如"编辑距离基础分"、"查询亲和分数"）。 */
  description: string
  /** 计入方式：add = 加权加分项；multiply = 乘法系数项（如长度比率、溢出惩罚、抑制因子）。 */
  kind: 'add' | 'multiply'
}

/**
 * 第三方插件翻译目录：嵌套结构 `{ plugin: { <pluginId>: { … } } }`，
 * 由 `i18n_get_plugin_translations` 下发，前端以 vue-i18n 命名空间方式合并。
 */
export type PluginTranslationCatalog = Record<string, Record<string, unknown>>
