import type { ComponentInfo, HelpNavigation } from '../bridge/contract'
import type { SidebarCategory } from './settingsSidebar'
import { pipelineGroupLabel } from './pipelineGroups'
import { resolveText } from '../i18n'

/** 助手路径里的固定文案：由调用方按当前界面语言注入。 */
export interface AssistantNavigationLabels {
  /** 设置页标题（路径起点）。 */
  root: string
  /** 侧边栏「插件」文案。 */
  plugins: string
  /** 第三方插件行的「配置」按钮文案。 */
  config: string
}

/**
 * 生成助手用的完整点击链：按分类实际的渲染形态拼到组件。
 * 参数：categories - 侧边栏分类（含分类内组件）；components - 全部组件（补上不在分类里的第三方组件）；
 * labels - 路径固定文案；t - 翻译函数（用于管道类型标签页）。
 * 返回：每个组件一条路径与字段分组形态；未知管道类型回落为分类 → 组件。
 */
export function buildAssistantNavigation(
  categories: SidebarCategory[],
  components: ComponentInfo[],
  labels: AssistantNavigationLabels,
  t: (key: string) => string,
): HelpNavigation[] {
  const entries: HelpNavigation[] = []
  const covered = new Set<string>()
  for (const category of categories) {
    for (const component of category.components ?? []) {
      covered.add(component.componentId)
      const steps = [labels.root, category.label]
      // 搜索管道分类在分类与组件之间还有一层组件类型标签页
      if (category.type === 'pipeline') {
        const groupLabel = pipelineGroupLabel(component.componentType, t)
        if (groupLabel) steps.push(groupLabel)
      }
      steps.push(resolveText(component.componentName))
      entries.push({
        componentId: component.componentId,
        path: steps.join(' › '),
        groupTabs: category.groupTabs ?? false,
      })
    }
  }
  // 插件页只列 ComponentType.Plugin；candidate-registry 这类内部组件没有设置入口，
  // 不能给它编一条插件页路径。
  for (const component of components) {
    if (covered.has(component.componentId)) continue
    if (component.componentType !== 'Plugin') continue
    entries.push({
      componentId: component.componentId,
      path: `${labels.root} › ${labels.plugins} › ${resolveText(component.componentName)} › ${labels.config}`,
      groupTabs: false,
    })
  }
  return entries
}
