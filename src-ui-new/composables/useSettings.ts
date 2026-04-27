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
  async function openSettings() {
    const existing = await WebviewWindow.getByLabel('settings')
    if (existing) {
      await existing.setFocus()
      return
    }

    new WebviewWindow('settings', {
      url: '/#/settings',
      title: '设置',
      width: 800,
      height: 600,
      resizable: true,
      center: true,
    })
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
