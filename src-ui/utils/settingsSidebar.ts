import type { ComponentInfo } from '../bridge/contract'

export interface SidebarCategory {
  key: string
  label: string
  icon: string
  type: 'category' | 'pipeline' | 'plugins' | 'tabs' | 'static' | 'plugin' | 'debug'
  components?: ComponentInfo[]
  items?: SidebarCategory[]
}

/** 按 (priority, componentId) 升序排列，确保设置页 tab 顺序稳定。 */
function sortByPriority(a: ComponentInfo, b: ComponentInfo): number {
  return (a.priority ?? 50) - (b.priority ?? 50) || a.componentId.localeCompare(b.componentId)
}


export function buildSidebarItems(
  components: ComponentInfo[],
  isDebugMode: boolean,
): SidebarCategory[] {
  const core = components.filter(
    (c) =>
      c.componentType === 'Core' &&
      c.componentId !== 'candidate-registry' &&
      c.componentId !== 'bias-config' &&
      c.componentId !== 'appearance-config',
  ).sort(sortByPriority)
  const appearance = components.filter((c) => c.componentId === 'appearance-config').sort(sortByPriority)
  const pipeline = components.filter((c) =>
    ['DataSource', 'KeywordOptimizer', 'KeywordInjector', 'SearchEngine', 'ScoreBooster', 'ActionExecutor', 'BiasRule'].includes(
      c.componentType,
    ),
  ).sort(sortByPriority)
  const plugins = components.filter((c) => c.componentType === 'Plugin')

  const items: SidebarCategory[] = [
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
  ]

  // 仅在调试模式开启时显示
  if (isDebugMode) {
    items.push({
      key: 'category_debug',
      label: '调试工具',
      icon: 'bug',
      type: 'debug' as const,
    })
  }

  items.push({ key: 'category_about', label: '关于', icon: 'info', type: 'static' })
  return items
}
