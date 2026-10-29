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

/** 隐藏方式 —— 与后端 exclusion-config 的规则 kind 同词表。 */
export type ExclusionKind = 'exact' | 'path_prefix' | 'contains'

/** 隐藏方式取值（右键「不再显示」面板展示用）：三项取值均由后端从候选项推导。 */
export interface ExcludeOptions {
  /** 精确匹配目标（文件候选即完整路径）。 */
  exact: string
  /** 目录前缀目标；null 表示该候选没有文件夹语义（面板关闭目录方式）。 */
  folder: string | null
  /** 关键字包含的默认取值（候选显示名，可在面板中改写）。 */
  keyword: string
}

/** 隐藏候选项的请求载荷（字段与后端 ExcludeCandidatePayload 一一对应）。 */
export interface ExcludeCandidatePayload {
  /** 目标候选项 ID。 */
  candidateId: number
  /** 前端持有的候选缓存世代（后端校验后拒绝过期载荷，防止缓存刷新后隐藏错目标）。 */
  candidateGeneration: number
  /** 隐藏方式。 */
  kind: ExclusionKind
  /** 关键字包含的取值（仅 contains 使用，空串由后端拒绝）。 */
  keyword: string
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

/** 宿主会话子形态（HostSessionStateEvent.view）：none=无会话；search=默认搜索；inlineParam/paramPanel=宿主行内参数流程。 */
export type HostView = 'none' | 'search' | 'inlineParam' | 'paramPanel'

/** 插件会话子形态（PluginSessionStateEvent.view）：pluginPanel=行内面板；pluginImmersive=全页面面板。 */
export type PluginView = 'pluginPanel' | 'pluginImmersive'

/** 前端本地镜像谓词（PluginSessionStateEvent.inputMatch）：
 *  - `keywords`：同一对象内自带触发词，前端可本地镜像（见 queryStillInPanel）；
 *  - `custom`：由插件 match_query 自决，前端无本地谓词（非空输入一律视为仍属本面板，粘性）。 */
export type InputMatch =
  | { model: 'keywords'; triggerKeywords: string[] }
  | { model: 'custom' }

/** 插件面板渲染载荷（PluginSessionStateEvent.panelContent；热键唤醒推送携带，常规查询路径该键不出现）。 */
export interface PluginPanelContent {
  panelType: string
  data: unknown
  actions: ResultAction[]
}

/** 宿主会话事件（默认搜索/行内参数/参数面板/会话结束）：无插件归属、交互契约与触发模型。 */
export interface HostSessionStateEvent {
  kind: 'host'
  /** 会话代际：归属/形态变化时递增（前端单调递增更新，随 confirm 回传校验）。 */
  generation: number
  view: HostView
}

/** 插件会话事件（查询路由命中 / 热键唤醒）。 */
export interface PluginSessionStateEvent {
  kind: 'plugin'
  /** 会话代际：归属/形态变化时递增（前端单调递增更新，随 confirm 回传校验）。 */
  generation: number
  /** 会话归属插件 ID。 */
  pluginId: string
  view: PluginView
  /** 前端本地镜像谓词唯一来源（见 queryStillInPanel）；null = 无本地谓词。 */
  inputMatch: InputMatch | null
  /** 插件面板交互契约（含按键映射）；插件会话必然存在。 */
  interaction: PanelInteraction
  /** 面板渲染载荷：对象 = 热键唤醒推送；null = 常规查询路径（载荷随 bridge_query 响应下发）。该键恒存在（后端序列化不跳过字段）。 */
  panelContent: PluginPanelContent | null
}

/** 会话状态事件 payload —— 整个会话系统的唯一事件（后端 Dispatcher 推送，kind 判别联合）。 */
export type SessionStateEvent = HostSessionStateEvent | PluginSessionStateEvent

export type BridgeQueryResponse =
  | { mode: 'search'; generation: number; candidateGeneration: number; results: ListItem[]; panelType: null; panelData: null; panelActions: null; inlineParam: null }
  | { mode: 'plugin_panel'; generation: number; candidateGeneration: number; results: ListItem[]; panelType: string; panelData: unknown; panelActions: ResultAction[]; inlineParam: null }
  | { mode: 'plugin_immersive'; generation: number; candidateGeneration: number; results: ListItem[]; panelType: string; panelData: unknown; panelActions: ResultAction[]; inlineParam: null }
  | { mode: 'inline_param'; generation: number; candidateGeneration: number; results: never[]; panelType: null; panelData: null; panelActions: null; inlineParam: InlineParamData }

/**
 * 确认请求 —— `bridge_confirm` 的 IPC 载荷（与后端 ConfirmRequestPayload tagged union 对齐）：
 * - `candidate`：宿主候选确认（默认搜索执行 / 插件面板默认动作）；
 * - `pluginAction`：插件面板动作（面板按键契约 Custom / GotoPanel 回插件）。
 */
/**
 * 查询请求 —— `bridge_query` 的 IPC 载荷（与后端 QueryPayload 对齐）。
 * `panelPluginId` 有值时 = 沉浸式面板数据通道（面板内查询，显式指定目标插件）。
 */
export interface QueryPayload {
  rawQuery: string
  confirm: boolean
  panelPluginId: string | null
}

export type ConfirmRequest =
  | {
      kind: 'candidate'
      candidateId: number
      actionId: string
      queryText: string
      userArgs?: string[]
      /** 会话代际：最后一次观测到的代际，后端据此拒绝过期确认（必填）。 */
      generation: number
      /** 候选缓存世代：最后一次观测到的世代（查询响应下发），后端校验防 id 漂移。 */
      candidateGeneration: number
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
  | { status: 'executed'; generation: number; candidateGeneration: number }
  | {
      status: 'enterParamPanel'
      candidateId: number
      userArgCount: number
      generation: number
      candidateGeneration: number
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
  | 'ExclusionRule'
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
  | { kind: 'multiselect' }
  | { kind: 'path'; mode: 'file' | 'directory' }
  | { kind: 'color' }
  | { kind: 'image'; accept: string[]; maxSize: number | null }
  | { kind: 'font'; action: string; component: string | null }
  | { kind: 'hotkey'; requireModifier: boolean }
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

/** 字段可见性条件 — 与同级字段 JSON 值全等比较，命中才显示该字段（仅展示层）。 */
export interface VisibleWhen {
  field: string
  value: unknown
}

export interface FieldUiMetadata {
  pointer: string
  label: string
  description: string
  /** 字段补充说明（i18n key）：作用范围/生效时机/特殊限制，界面以次要说明展示。 */
  note: string | null
  group: string | null
  order: number
  visible: boolean
  readOnly: boolean
  /** 敏感字段（如 API Key）：宿主读取其值用于 AI 助手时会遮蔽；旧版插件 schema 可能不携带该字段。仅契约镜像（后端脱敏使用，前端暂不消费）。 */
  sensitive?: boolean
  visibleWhen: VisibleWhen | null
  widget: WidgetHint | null
  action: FieldAction | null
  detailAction: DetailActionDef | null
  /** 拖放投放声明；null（或旧版插件缺省该字段）表示本字段不接受拖放。 */
  drop?: DropSpec | null
}

/**
 * 拖放投放声明 — 外部拖入的路径写进本字段的哪个位置。
 * 拖目录写入 `pathField`；拖文件写入其父目录，并把 `*.<扩展名>` 并入 `patternField`（非 null 时）。
 * 标量 `path` 字段无需声明，按 widget 的 mode 推断。
 */
export interface DropSpec {
  pathField: string
  patternField: string | null
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

// ── AI 助手 ──

/** 设置字段检索命中（`help_search`）。 */
export interface HelpHit {
  componentId: string
  fieldKey: string
  label: string
}

/** 助手答复中引用到的设置位置。 */
export interface HelpReference {
  componentId: string
  componentName: string
  fieldKey: string
  fieldLabel: string
}

/** 助手答复（`help_ask`）。 */
export interface HelpAnswer {
  answer: string
  /** 工具调用轮里模型写下的过渡正文，按独立助手消息展示。 */
  intermediates: string[]
  references: HelpReference[]
}

/** `help_ask` 的设置页路径项：组件 id → 完整点击链与字段分组形态。 */
export interface HelpNavigation {
  componentId: string
  /** 完整点击链，含分类、组件类型标签页与组件名。 */
  path: string
  /** 组件内字段分组是否渲染为标签页；false 表示页内分组标题。 */
  groupTabs: boolean
}

/** `help_ask` 的历史消息：本次提问之前的对话（宿主侧另有权威裁剪）。 */
export interface HelpMessage {
  role: 'user' | 'assistant'
  content: string
}

// ── AI 助手调试轨迹（`debug_get_agent_traces`） ──

/** 轨迹中的一条消息 —— 首轮下发给模型的消息序列中的一项。 */
export interface AgentTraceMessage {
  /** 消息角色。 */
  role: 'system' | 'user' | 'assistant' | 'tool'
  /** 消息正文。 */
  content: string
}

/** 轨迹中的一次工具调用。 */
export interface AgentTraceToolCall {
  /** 工具名。 */
  name: string
  /** 模型给出的参数原文（JSON 文本）。 */
  arguments: string
  /** 工具回喂给模型的文本。 */
  result: string
}

/** 轨迹中的一轮模型调用。 */
export interface AgentTraceRound {
  /** 模型本轮返回的正文（纯工具调用轮为空串）。 */
  content: string
  /** 本轮模型请求的工具调用及其执行结果。 */
  toolCalls: AgentTraceToolCall[]
}

/** 一次提问的调试轨迹（宿主内存里保留最近若干条，最新在前）。 */
export interface AgentTrace {
  /** 记录时刻（本地时间 `YYYY-MM-DD HH:MM:SS`）。 */
  time: string
  /** 本次使用的模型全局 id（含 provider 前缀）。 */
  modelId: string
  /** 首轮随请求下发的工具名清单。 */
  tools: string[]
  /** 首轮下发的完整上下文：系统提示词 + 历史对话 + 当前问题。 */
  messages: AgentTraceMessage[]
  /** 每次模型调用的输出与随后的工具执行。 */
  rounds: AgentTraceRound[]
  /** 模型调用失败信息；`null` 表示本次提问未发生模型调用错误。 */
  error: string | null
}
