import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { open as shellOpen } from '@tauri-apps/plugin-shell'
import type { BridgeQueryResponse, ConfirmRequest, ConfirmResponse, ComponentInfo, ComponentSchema, ConfigActionDef, ConfigActionPayload, SearchTimingResult, IndexTimingResult, SearchDetailItem, PluginTranslationCatalog } from './contract'
export interface BridgeError {
  code: string
  message: string
  details: unknown | null
  componentId: string | null
  traceId: string
}

let onError: ((error: BridgeError) => void) | null = null

export function registerErrorHandler(handler: (error: BridgeError) => void) {
  onError = handler
}

function tryExtractBridgeError(e: unknown): BridgeError {
  if (typeof e === 'object' && e !== null && 'code' in e && 'message' in e) {
    return e as BridgeError
  }
  return {
    code: 'INTERNAL_ERROR',
    message: typeof e === 'string' ? e : String(e),
    traceId: '',
    details: null,
    componentId: null,
  }
}

async function invokeCommand<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args)
  } catch (e) {
    const error = tryExtractBridgeError(e)
    onError?.(error)
    throw error
  }
}

// ---- 搜索 & 会话 ----

/**
 * 发起查询。
 * @param confirm 是否由用户显式确认（Enter）触发：false=预览/路由查询（OnEnter 模式返回 ready），true=确认查询（OnEnter 模式执行动作）。
 */
export function bridgeQuery(rawQuery: string, confirm: boolean): Promise<BridgeQueryResponse> {
  return invokeCommand<BridgeQueryResponse>('bridge_query', { rawQuery, confirm })
}

export function bridgeConfirm(payload: ConfirmRequest): Promise<ConfirmResponse> {
  return invokeCommand<ConfirmResponse>('bridge_confirm', { payload })
}

export function bridgeWake(): Promise<void> {
  return invokeCommand<void>('bridge_wake')
}

export function bridgeReset(): Promise<void> {
  return invokeCommand<void>('bridge_reset')
}

export function bridgeRefreshCandidates(): Promise<number> {
  return invokeCommand<number>('bridge_refresh_candidates')
}

export function bridgeGetCandidatesCount(): Promise<number> {
  return invokeCommand<number>('bridge_get_candidates_count')
}

export function bridgeHideWindow(): Promise<void> {
  return invokeCommand<void>('bridge_hide_window')
}


// ---- 配置管理 ----

export function configGetVersion(): Promise<string> {
  return invokeCommand<string>('config_get_version')
}

export function configGetAllComponents(): Promise<ComponentInfo[]> {
  return invokeCommand<ComponentInfo[]>('config_get_all_components')
}

export function configGetSchema(componentId: string): Promise<ComponentSchema> {
  return invokeCommand<ComponentSchema>('config_get_schema', { componentId })
}

export function configGetSettings(componentId: string): Promise<unknown> {
  return invokeCommand<unknown>('config_get_settings', { componentId })
}

export function configApplySettings(componentId: string, settings: unknown): Promise<void> {
  return invokeCommand<void>('config_apply_settings', { componentId, settings })
}

export function configResetSettings(componentId: string): Promise<void> {
  return invokeCommand<void>('config_reset_settings', { componentId })
}

export function configSetEnabled(componentId: string, enabled: boolean): Promise<void> {
  return invokeCommand<void>('config_set_enabled', { componentId, enabled })
}

export function configGetActions(componentId: string): Promise<ConfigActionDef[]> {
  return invokeCommand<ConfigActionDef[]>('config_get_actions', { componentId })
}

export function configExecuteAction(
  componentId: string,
  action: string,
  params?: unknown,
): Promise<unknown> {
  const payload: ConfigActionPayload = { componentId, action, params }
  return invokeCommand<unknown>('config_execute_action', { payload })
}

// ---- 资源管理 ----

export function resourceGet(resourceId: string): Promise<string> {
  return invokeCommand<string>('resource_get', { resourceId })
}

export function resourceUpload(filePath: string, resourceId: string, maxSize?: number): Promise<string> {
  return invokeCommand<string>('resource_upload', { payload: { filePath, resourceId, maxSize } })
}

// ---- Plugin Inspector ----

export function inspectorGetState(): Promise<import('./contract').InspectorStateResponse> {
  return invokeCommand('inspector_get_state')
}

// ---- Third-Party Plugin Management ----

/** 插件运行状态（对应后端 PluginRuntimeState，snake_case 序列化）。 */
export type PluginRuntimeState =
  | 'starting'
  | 'running'
  | 'stopped'
  | 'crashed'
  | 'error'
  | 'unknown'

export interface InstalledPluginInfo {
  pluginId: string
  name: string
  version: string
  description: string
  author: string
  state: PluginRuntimeState
  enabled: boolean
  /** 插件种类：内置或第三方（内置条目由后端 plugin_list 合并填充）。 */
  kind: 'builtin' | 'third-party'
  priority: number
  componentIds: string[]
  /** 全局唤醒快捷键（如 "Ctrl+E"）：非 null = 声明热键唤醒；null = 未声明。形态判定以 mode 为准。 */
  hotkey: string | null
  /** 插件显示图标（data URL，如 "data:image/png;base64,..."），null 表示无图标。 */
  icon: string | null
  /** 插件形态：'inline' = 行内插件；'panel' = 完全插件模式（trigger 类型）。 */
  mode: 'inline' | 'panel'
}

/**
 * 弹出 .zip 文件选择框（第三方插件安装用）。
 * 注意：tauri-plugin-dialog 的 directory 模式忽略 filters（Windows 文件夹选择器不可选文件），
 * 文件与目录选择必须拆成两个入口。
 * @param filterLabel 文件过滤器的显示名称（由调用方传入 i18n 文案）
 * @returns 选中的路径；用户取消时为 null
 */
export function pickPluginZip(filterLabel: string): Promise<string | null> {
  return open({
    multiple: false,
    directory: false,
    filters: [{ name: filterLabel, extensions: ['zip'] }],
  })
}

/**
 * 弹出插件目录选择框（第三方插件安装用，开发期未打包插件）。
 * @param filterLabel 对话框标题文案（由调用方传入 i18n 文案）
 * @returns 选中的目录；用户取消时为 null
 */
export function pickPluginDir(filterLabel: string): Promise<string | null> {
  return open({
    multiple: false,
    directory: true,
    title: filterLabel,
  })
}

export function pluginList(): Promise<InstalledPluginInfo[]> {
  return invokeCommand<InstalledPluginInfo[]>('plugin_list')
}

/** 插件热键唤醒（前端驱动）：搜索栏唤起后前端匹配插件声明热键时调用。 */
export function bridgeWakePlugin(pluginId: string): Promise<void> {
  return invokeCommand<void>('bridge_wake_plugin', { pluginId })
}

export function pluginGetManifest(pluginId: string): Promise<unknown> {
  return invokeCommand<unknown>('plugin_get_manifest', { pluginId })
}

/** 第三方插件 manifest 的完整 JSON 形状（与 plugin-protocol Manifest 序列化结果一致）。
 *  可选段在 Rust 侧为 Option 且序列化恒为 null（无 skip_serializing_if），故此处用 `| null` 而非可选 `?:`。 */
export interface PluginManifest {
  plugin: {
    id: string
    name: string
    version: string
    description: string
    author: string
    homepage: string | null
    license: string | null
    minHostVersion: string
  }
  runtime: {
    command: string
    args: string[]
    startupTimeout: number
    autoRestart: boolean
    maxRestart: number
  }
  components: {
    provides: string[]
  }
  ui: {
    panelEntry: string | null
    settingsEntry: string | null
    resultItemEntry: string | null
  } | null
  icon: {
    path: string
  } | null
}

/** 插件详情：插件级基础视图（与 plugin_list 同构，后端 flatten 展开）+ 详情专属字段。 */
export type PluginDetail = InstalledPluginInfo & {
  triggerKeywords: string[]
  supportedOs: string[]
  manifest: PluginManifest | null
}

export function pluginGetDetail(pluginId: string): Promise<PluginDetail> {
  return invokeCommand<PluginDetail>('plugin_get_detail', { pluginId })
}

export function pluginReload(pluginId: string): Promise<void> {
  return invokeCommand<void>('plugin_reload', { pluginId })
}

export function pluginUninstall(pluginId: string): Promise<void> {
  return invokeCommand<void>('plugin_uninstall', { pluginId })
}

export function pluginInstallLocal(filePath: string): Promise<InstalledPluginInfo> {
  return invokeCommand<InstalledPluginInfo>('plugin_install_local', { filePath })
}

export function pluginSetEnabled(pluginId: string, enabled: boolean): Promise<void> {
  return invokeCommand<void>('plugin_set_enabled', { pluginId, enabled })
}

export function pluginGetLogs(pluginId: string, tailLines?: number): Promise<string[]> {
  return invokeCommand<string[]>('plugin_get_logs', { pluginId, tailLines })
}

export interface CliInfo {
  host: string
  port: number
  token: string
}

export function cliGetInfo(): Promise<CliInfo> {
  return invokeCommand<CliInfo>('cli_get_info')
}

// ---- Debug Tools ----

export function debugTestSearchTime(query: string): Promise<SearchTimingResult> {
  return invokeCommand('debug_test_search_time', { query })
}

export function debugTestIndexTime(): Promise<IndexTimingResult> {
  return invokeCommand('debug_test_index_time')
}

export function debugGetSearchKeys(name: string): Promise<string[]> {
  return invokeCommand('debug_get_search_keys', { name })
}

export function debugSearchDetail(query: string): Promise<SearchDetailItem[]> {
  return invokeCommand('debug_search_detail', { query })
}

// ---- i18n ----

/**
 * 获取指定语言下所有已加载第三方插件的翻译目录（嵌套结构，命名空间 `plugin.<id>.`）。
 * @param lang 语言码（zh-Hans / zh-Hant / en）
 */
export function i18nGetPluginTranslations(lang: string): Promise<PluginTranslationCatalog> {
  return invokeCommand<PluginTranslationCatalog>('i18n_get_plugin_translations', { lang })
}

// ---- 系统集成 ----

/**
 * 用系统默认浏览器打开外链。
 * webview 内 `target=_blank` 只会打开空白窗口，须走 shell 插件；调用失败时 reject。
 */
export function openExternal(url: string): Promise<void> {
  return shellOpen(url)
}
