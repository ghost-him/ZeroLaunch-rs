import { WebviewWindow } from '@tauri-apps/api/webviewWindow'
import type { ComponentInfo } from '../bridge/contract'

export interface SidebarItem {
  key: string
  label: string
  icon: string
  type: 'list' | 'tabs' | 'static'
  items?: ComponentInfo[]
}

export function useSettings() {
  /**
   * Show the settings window (already created by Rust backend at startup).
   * The Rust backend manages the window lifecycle; frontend only shows/hides.
   */
  async function openSettings() {
    const win = await WebviewWindow.getByLabel('setting_window')
    if (win) {
      await win.show()
      await win.setFocus()
    }
  }

  function buildSidebarItems(components: ComponentInfo[]): SidebarItem[] {
    const core = components.filter((c) => c.componentType === 'Core')
    const pipeline = components.filter((c) =>
      ['DataSource', 'KeywordOptimizer', 'SearchEngine', 'ScoreBooster', 'ActionExecutor'].includes(
        c.componentType,
      ),
    )
    const plugins = components.filter((c) => c.componentType === 'Plugin')

    return [
      { key: 'core', label: '常规', icon: 'settings', type: 'list', items: core },
      { key: 'pipeline', label: '搜索管道', icon: 'search', type: 'tabs', items: pipeline },
      { key: 'plugins', label: '插件', icon: 'extension', type: 'list', items: plugins },
      { key: 'appearance', label: '外观', icon: 'palette', type: 'static' },
      { key: 'about', label: '关于', icon: 'info', type: 'static' },
    ]
  }

  return { openSettings, buildSidebarItems }
}
