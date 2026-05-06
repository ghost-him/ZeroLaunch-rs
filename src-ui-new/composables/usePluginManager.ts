import { usePluginStore } from '@/stores/plugin-store'
import calculatorPanelPlugin from '@/plugins/built-in/calculator-panel'

let builtinsLoaded = false

export function usePluginManager() {
  const pluginStore = usePluginStore()

  /** 加载内置插件（幂等） */
  async function loadBuiltinPlugins(): Promise<void> {
    if (builtinsLoaded) return

    await pluginStore.registerPlugin(calculatorPanelPlugin)
    builtinsLoaded = true
  }

  return {
    loadBuiltinPlugins,
  }
}
