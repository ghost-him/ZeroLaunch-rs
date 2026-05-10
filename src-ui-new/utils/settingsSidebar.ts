import type { ComponentInfo } from '../bridge/contract'

export interface SidebarCategory {
  key: string
  label: string
  icon: string
  type: 'category' | 'pipeline' | 'plugins' | 'tabs' | 'static' | 'plugin'
  components?: ComponentInfo[]
  items?: SidebarCategory[]
}

export function buildSidebarItems(components: ComponentInfo[]): SidebarCategory[] {
  const core = components.filter(
    (c) => c.componentType === 'Core' && c.componentId !== 'appearance',
  )
  const appearance = components.filter((c) => c.componentId === 'appearance')
  const pipeline = components.filter((c) =>
    ['DataSource', 'KeywordOptimizer', 'SearchEngine', 'ScoreBooster', 'ActionExecutor'].includes(
      c.componentType,
    ),
  )
  const plugins = components.filter((c) => c.componentType === 'Plugin')

  return [
    { key: 'category_core', label: '常规设置', icon: 'settings', type: 'tabs', components: core },
    { key: 'category_appearance', label: '外观设置', icon: 'palette', type: 'tabs', components: appearance },
    { key: 'category_pipeline', label: '搜索管道', icon: 'search', type: 'pipeline', components: pipeline },
    {
      key: 'category_plugins',
      label: '扩展插件',
      icon: 'extension',
      type: 'plugins',
      items: plugins.map(p => ({
        key: `plugin_${p.componentId}`,
        label: p.componentName,
        icon: 'plugin',
        type: 'plugin' as const,
        components: [p]
      }))
    },
    { key: 'category_about', label: '关于', icon: 'info', type: 'static' },
  ]
}
