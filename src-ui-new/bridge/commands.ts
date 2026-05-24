import { invoke } from '@tauri-apps/api/core'
import type { BridgeQueryResponse, ConfirmPayload, ComponentInfo, ComponentSchema, ConfigActionDef } from './contract'

// ---- 错误类型 ----

export interface BridgeError {
  code: string
  message: string
  details?: unknown
  componentId?: string
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

export function bridgeQuery(rawQuery: string): Promise<BridgeQueryResponse> {
  return invokeCommand<BridgeQueryResponse>('bridge_query', { rawQuery })
}

export function bridgeConfirm(payload: ConfirmPayload): Promise<void> {
  return invokeCommand<void>('bridge_confirm', { payload })
}

export function bridgeWake(): Promise<void> {
  return invokeCommand<void>('bridge_wake')
}

export function bridgeReset(): Promise<void> {
  return invokeCommand<void>('bridge_reset')
}

export function bridgeGetSessionMode(): Promise<string> {
  return invokeCommand<string>('bridge_get_session_mode')
}

export function bridgeRefreshCandidates(): Promise<number> {
  return invokeCommand<number>('bridge_refresh_candidates')
}

export function bridgeGetCandidatesCount(): Promise<number> {
  return invokeCommand<number>('bridge_get_candidates_count')
}


// ---- 配置管理 ----

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
  return invokeCommand<unknown>('config_execute_action', { componentId, action, params })
}

// ---- 资源管理 ----

export function resourceGet(resourceId: string): Promise<string> {
  return invokeCommand<string>('resource_get', { resourceId })
}

export function resourceUpload(filePath: string, purpose: string, maxSize?: number): Promise<string> {
  return invokeCommand<string>('resource_upload', { payload: { filePath, purpose, maxSize } })
}
