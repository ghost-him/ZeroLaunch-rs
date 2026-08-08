import type { ComponentInfo } from '../bridge/contract'

export interface SidebarCategory {
  key: string
  label: string
  icon: string
  type: 'pipeline' | 'tabs' | 'static' | 'debug' | 'plugins-page'
  components?: ComponentInfo[]
}

/** 按 (priority, componentId) 升序排列，确保设置页 tab 顺序稳定。 */
function sortByPriority(a: ComponentInfo, b: ComponentInfo): number {
  return (a.priority ?? 50) - (b.priority ?? 50) || a.componentId.localeCompare(b.componentId)
}


export function buildSidebarItems(
  components: ComponentInfo[],
  isDebugMode: boolean,
  t: (key: string) => string,
): SidebarCategory[] {
  const core = components.filter(
    (c) =>
      c.componentType === 'Core' &&
      c.componentId !== 'candidate-registry' &&
      c.componentId !== 'bias-config' &&
      c.componentId !== 'appearance-config' &&
      c.componentId !== 'icon-override-config',
  ).sort(sortByPriority)
  const appearance = components.filter(
    (c) => c.componentId === 'appearance-config' || c.componentId === 'icon-override-config',
  ).sort(sortByPriority)
  const pipeline = components.filter((c) =>
    ['DataSource', 'KeywordOptimizer', 'KeywordInjector', 'SearchEngine', 'ScoreBooster', 'ActionExecutor', 'BiasRule'].includes(
      c.componentType,
    ),
  ).sort(sortByPriority)

  const items: SidebarCategory[] = [
    { key: 'category_core', label: t('settings.sidebar.general'), icon: 'settings', type: 'tabs', components: core },
    { key: 'category_appearance', label: t('settings.sidebar.appearance'), icon: 'palette', type: 'tabs', components: appearance },
    { key: 'category_pipeline', label: t('settings.sidebar.pipeline'), icon: 'search', type: 'pipeline', components: pipeline },
    // 统一插件管理页（内置 + 第三方）：安装、运行状态与配置入口都在此页，不占侧边栏子项
    { key: 'category_plugins', label: t('settings.sidebar.plugins'), icon: 'extension', type: 'plugins-page' as const },
  ]

  // 仅在调试模式开启时显示
  if (isDebugMode) {
    items.push({
      key: 'category_debug',
      label: t('settings.sidebar.debug'),
      icon: 'bug',
      type: 'debug' as const,
    })
  }

  items.push({ key: 'category_about', label: t('settings.sidebar.about'), icon: 'info', type: 'static' })
  return items
}
