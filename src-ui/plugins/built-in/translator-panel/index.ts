import type { FrontendPlugin } from '@/plugins/types'
import TranslationPanel from './TranslationPanel.vue'
import TranslatorSettings from './TranslatorSettings.vue'

const translatorPanelPlugin: FrontendPlugin = {
  id: 'translator-panel',
  name: '翻译面板',
  version: '1.0.0',
  description: '内置翻译面板与设置，匹配后端 TranslatorPlugin',
  priority: 0,

  panelProvider: {
    matchType: 'translator',
    component: TranslationPanel,
  },

  settingsProvider: {
    matchComponentId: 'translator',
    component: TranslatorSettings,
  },
}

export default translatorPanelPlugin
