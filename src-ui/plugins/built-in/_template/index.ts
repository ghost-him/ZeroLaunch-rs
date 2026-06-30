import type { FrontendPlugin } from '@/plugins/types'

// 如果你的插件提供自定义面板，取消下面的 import 注释:
// import TemplatePanel from './TemplatePanel.vue'

const templatePlugin: FrontendPlugin = {
  id: '<plugin-id>',          // ← 改为你的唯一 ID
  name: '<Plugin 显示名称>',   // ← 改为你的显示名称
  version: '1.0.0',
  description: '<描述>',
  priority: 100,              // ← 数字越小越先加载

  // ---- 可选扩展点 (取消注释并按需配置) ----

  // 自定义面板渲染 (匹配后端 CustomPanel.panel_type)
  // panelProvider: {
  //   matchType: '<panel_type>',
  //   component: TemplatePanel,
  // },

  // 自定义结果项渲染 (覆盖默认 ResultItem)
  // resultItemProvider: {
  //   matchTypes: ['Path', 'App'],
  //   component: () => import('./CustomResultItem.vue'),
  //   priority: 50,
  // },

  // 为特定结果类型注入额外操作按钮
  // actionInjector: {
  //   matchTypes: ['Path'],
  //   getActions: (item) => [{
  //     id: 'my_action',
  //     label: 'My Action',
  //     icon: '',
  //     isDefault: false,
  //     shortcutKey: '',
  //   }],
  //   priority: 50,
  // },

  // 自定义设置面板 (覆盖 DynamicForm)
  // settingsProvider: {
  //   matchComponentId: '<component_id>',
  //   component: () => import('./CustomSettings.vue'),
  // },
}

export default templatePlugin
