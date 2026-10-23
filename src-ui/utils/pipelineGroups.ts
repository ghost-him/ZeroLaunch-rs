/** 搜索管道的组件类型标签页定义：设置页 tab 与助手导航共用同一份。 */
export interface PipelineGroup {
  pane: string
  type: string
  labelKey: string
  customToggle?: boolean
}

export const PIPELINE_GROUPS: PipelineGroup[] = [
  { pane: 'datasource', type: 'DataSource', labelKey: 'settings.dataSource' },
  { pane: 'processor', type: 'KeywordOptimizer', labelKey: 'settings.contentProcessor' },
  { pane: 'injector', type: 'KeywordInjector', labelKey: 'settings.keywordInjector' },
  { pane: 'bias', type: 'BiasRule', labelKey: 'settings.biasRule' },
  { pane: 'exclusion', type: 'ExclusionRule', labelKey: 'settings.exclusionRule' },
  { pane: 'searchengine', type: 'SearchEngine', labelKey: 'settings.searchEngine', customToggle: true },
  { pane: 'scorebooster', type: 'ScoreBooster', labelKey: 'settings.scoreBooster' },
  { pane: 'executor', type: 'ActionExecutor', labelKey: 'settings.actionExecutor' },
]

/** 按组件类型取管道标签页文案；未知类型返回 null。 */
export function pipelineGroupLabel(
  componentType: string,
  t: (key: string) => string,
): string | null {
  const group = PIPELINE_GROUPS.find((entry) => entry.type === componentType)
  return group ? t(group.labelKey) : null
}
