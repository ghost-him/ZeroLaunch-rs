import type { FrontendPlugin } from '@/plugins/types'
import CalculatorPanel from './CalculatorPanel.vue'

const calculatorPanelPlugin: FrontendPlugin = {
  id: 'calculator-panel',
  name: '计算器面板',
  version: '1.0.0',
  description: '内置计算器面板渲染，匹配后端 CalculatorPlugin 的 CustomPanel',
  priority: 0,

  panelProvider: {
    matchType: 'calculator',
    component: CalculatorPanel,
  },
}

export default calculatorPanelPlugin
