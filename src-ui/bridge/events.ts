import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { ConfigChangedPayload, ConfigErrorPayload, InstallationEventPayload, PluginEventPayload } from './contract'

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

export function onSessionReset(
  callback: () => void,
): Promise<UnlistenFn> {
  return listen('session-reset', () => {
    callback()
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
