import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { ConfigChangedPayload, ConfigErrorPayload, InstallationEventPayload, SessionStateEvent, PluginEventPayload } from './contract'

export function onConfigChanged(
  callback: (payload: ConfigChangedPayload) => void,
): Promise<UnlistenFn> {
  return listen<ConfigChangedPayload>('config-changed', (event) => {
    callback(event.payload)
  })
}

export function onConfigError(
  callback: (payload: ConfigErrorPayload) => void,
): Promise<UnlistenFn> {
  return listen<ConfigErrorPayload>('config-error', (event) => {
    callback(event.payload)
  })
}

export function onInstallationEvent(
  callback: (payload: InstallationEventPayload) => void,
): Promise<UnlistenFn> {
  return listen<InstallationEventPayload>('installation-event', (event) => {
    callback(event.payload)
  })
}

/** 后端会话投影变化时推送的会话状态事件（唯一会话事件通道，含会话结束 presentation:'none'）。 */
export function onSessionState(
  callback: (payload: SessionStateEvent) => void,
): Promise<UnlistenFn> {
  return listen<SessionStateEvent>('session-state', (event) => {
    callback(event.payload)
  })
}

export function onPluginInstalled(
  callback: (payload: PluginEventPayload) => void,
): Promise<UnlistenFn> {
  return listen<PluginEventPayload>('plugin-installed', (event) => {
    callback(event.payload)
  })
}

export function onPluginUninstalled(
  callback: (payload: PluginEventPayload) => void,
): Promise<UnlistenFn> {
  return listen<PluginEventPayload>('plugin-uninstalled', (event) => {
    callback(event.payload)
  })
}

export function onInspectorStateUpdated(
  callback: () => void,
): Promise<UnlistenFn> {
  return listen('inspector-state-updated', () => {
    callback()
  })
}

/** 后端唤出搜索栏窗口时推送（热键/托盘）。前端据此恢复输入焦点——窗口隐藏再显示时 DOM 焦点保留在旧元素上，需显式归还给搜索框。 */
export function onShowWindow(callback: () => void): Promise<UnlistenFn> {
  return listen('show_window', () => {
    callback()
  })
}
