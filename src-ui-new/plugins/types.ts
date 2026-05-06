import type { Component } from 'vue'
import type { ListItem, ResultAction } from '@/bridge/contract'

/** 前端插件完整契约 */
export interface FrontendPlugin {
  /** 元数据 */
  id: string
  name: string
  version: string
  description: string

  /** 生命周期 */
  onInit?: () => Promise<void>
  onDestroy?: () => Promise<void>

  /** 面板渲染：匹配后端 CustomPanel.panel_type */
  panelProvider?: PanelProvider

  /** 结果项渲染：自定义特定结果的显示方式 */
  resultItemProvider?: ResultItemProvider

  /** 结果项额外动作：为特定结果类型注入额外操作按钮 */
  actionInjector?: ActionInjector

  /** 自定义设置面板：覆盖 DynamicForm */
  settingsProvider?: SettingsProvider
}

/** 面板渲染提供者 — 用于 plugin_panel / plugin_immersive 两种形态 */
export interface PanelProvider {
  /** 匹配后端 CustomPanel.panel_type */
  matchType: string
  /** Vue 组件 (props: { data: unknown, actions: ResultAction[] }) */
  component: Component
}

/** 结果项渲染提供者 — 替换默认 ResultItem */
export interface ResultItemProvider {
  /** 匹配的目标类型 (与 ListItem.targetType 对应) */
  matchTypes: string[]
  /** 自定义渲染组件 (props: { item: ListItem, selected: boolean, index: number }) */
  component: Component
  /** 优先级，数字越小越优先（内置默认渲染器优先级 = 100） */
  priority: number
}

/** 动作注入器 — 为特定结果类型动态添加操作按钮 */
export interface ActionInjector {
  /** 匹配的目标类型 */
  matchTypes: string[]
  /** 返回需要注入的额外动作 */
  getActions: (item: ListItem) => ResultAction[]
  /** 优先级 */
  priority: number
}

/** 自定义设置面板提供者 — 覆盖 DynamicForm */
export interface SettingsProvider {
  /** 匹配后端 Configurable::component_id() */
  matchComponentId: string
  /** 自定义设置 Vue 组件 (props: { currentSettings: unknown, onSave: (s: unknown) => void }) */
  component: Component
}
