import { invoke } from '@tauri-apps/api/core'
import type { BridgeQueryResponse, ConfirmPayload, ComponentInfo, ComponentSchema, ConfigActionDef } from './contract'

// ---- 搜索 & 会话 ----

export function bridgeQuery(rawQuery: string): Promise<BridgeQueryResponse> {
  return invoke<BridgeQueryResponse>('bridge_query', { rawQuery })
}

export function bridgeConfirm(payload: ConfirmPayload): Promise<void> {
  return invoke<void>('bridge_confirm', { payload })
}

export function bridgeWake(): Promise<void> {
  return invoke<void>('bridge_wake')
}

export function bridgeReset(): Promise<void> {
  return invoke<void>('bridge_reset')
}

export function bridgeGetSessionMode(): Promise<string> {
  return invoke<string>('bridge_get_session_mode')
}

export function bridgeRefreshCandidates(): Promise<number> {
  return invoke<number>('bridge_refresh_candidates')
}

export function bridgeGetCandidatesCount(): Promise<number> {
  return invoke<number>('bridge_get_candidates_count')
}

// ---- 配置管理 ----

export function configGetAllComponents(): Promise<ComponentInfo[]> {
  return invoke<ComponentInfo[]>('config_get_all_components')
}

export function configGetSchema(componentId: string): Promise<ComponentSchema> {
  return invoke<ComponentSchema>('config_get_schema', { componentId })
}

export function configGetSettings(componentId: string): Promise<unknown> {
  return invoke<unknown>('config_get_settings', { componentId })
}

export function configApplySettings(componentId: string, settings: unknown): Promise<void> {
  return invoke<void>('config_apply_settings', { componentId, settings })
}

export function configResetSettings(componentId: string): Promise<void> {
  return invoke<void>('config_reset_settings', { componentId })
}

export function configSetEnabled(componentId: string, enabled: boolean): Promise<void> {
  return invoke<void>('config_set_enabled', { componentId, enabled })
}

export function configGetActions(componentId: string): Promise<ConfigActionDef[]> {
  return invoke<ConfigActionDef[]>('config_get_actions', { componentId })
}

export function configExecuteAction(componentId: string, action: string): Promise<unknown> {
  return invoke<unknown>('config_execute_action', { componentId, action })
}
