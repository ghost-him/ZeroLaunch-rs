import type { FrontendPlugin } from '@/plugins/types'
import SmartTargetPanel from './SmartTargetPanel.vue'

/**
 * 智能目标面板：渲染后端 path-detect / url-detect 两个内置检测插件共用的 CustomPanel。
 * 两者 panel_type 均为 'smart-target'，前端只需一个面板实现（渲染差异由 data.kind 决定）。
 */
const smartTargetPlugin: FrontendPlugin = {
  id: 'smart-target',
  name: '智能目标面板',
  version: '1.0.0',
  description: '内置路径/网址检测面板渲染，匹配后端 path-detect / url-detect 的 CustomPanel',
  priority: 0,

  panelProvider: {
    matchType: 'smart-target',
    component: SmartTargetPanel,
  },
}

export default smartTargetPlugin
