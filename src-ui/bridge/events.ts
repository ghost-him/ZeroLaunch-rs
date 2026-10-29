import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { getCurrentWebview, type DragDropEvent } from '@tauri-apps/api/webview'
import type { ConfigChangedPayload, SessionStateEvent, PluginEventPayload } from './contract'

/**
 * 监听当前 webview 的原生文件拖放事件（enter / over / drop / leave）。
 * 原生拖放开启时 webview 不投递 HTML5 drop 事件，路径只能从这里取得；仅设置窗使用。
 */
export function onFileDragDrop(
  callback: (payload: DragDropEvent) => void,
): Promise<UnlistenFn> {
  return getCurrentWebview().onDragDropEvent((event) => {
    callback(event.payload)
  })
}

export function onConfigChanged(
  callback: (payload: ConfigChangedPayload) => void,
): Promise<UnlistenFn> {
  return listen<ConfigChangedPayload>('config-changed', (event) => {
    callback(event.payload)
  })
}

/** 后端会话投影变化时推送的会话状态事件（唯一会话事件通道，含会话结束 { kind:'host', view:'none' }）。 */
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

/** 后端系统主题变化时推送（Tauri ThemeChanged 事件广播），供 system 模式跟随系统主题。 */
export function onSystemThemeChanged(callback: (isDark: boolean) => void): Promise<UnlistenFn> {
  return listen<string>('system-theme-changed', (event) => {
    callback(event.payload === 'dark')
  })
}

/** 后端唤出搜索栏窗口时推送（热键/托盘）。前端据此恢复输入焦点——窗口隐藏再显示时 DOM 焦点保留在旧元素上，需显式归还给搜索框。 */
export function onShowWindow(callback: () => void): Promise<UnlistenFn> {
  return listen('show_window', () => {
    callback()
  })
}
