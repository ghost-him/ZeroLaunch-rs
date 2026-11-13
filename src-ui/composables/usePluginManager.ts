import { usePluginStore } from '@/stores/plugin-store'
import type { FrontendPlugin } from '@/plugins/types'

interface GlobEntry {
  default: FrontendPlugin
}

let builtinsLoaded = false

/// 第三方插件界面已迁移到 GPUI 宿主进程内的 gpui-shell；WebView 只保留内置插件。
export function usePluginManager() {
  const pluginStore = usePluginStore()

  async function loadBuiltinPlugins(): Promise<void> {
    if (builtinsLoaded) return

    const modules = import.meta.glob<GlobEntry>(
      '/src-ui/plugins/built-in/*/index.ts',
      { eager: true },
    )

    const entries = Object.entries(modules)
      .filter(([path]) => !path.includes('/_template/'))
      .map(([path, mod]) => ({ plugin: mod.default, path }))
      .sort((a, b) => (a.plugin.priority ?? 50) - (b.plugin.priority ?? 50))

    for (const { plugin, path } of entries) {
      try {
        pluginStore.registerPlugin(plugin)
      } catch (err) {
        console.error(`[PluginManager] 注册内置插件失败: ${path}`, err)
      }
    }

    builtinsLoaded = true
  }

  /// 兼容旧调用点：第三方插件不再注册 ESM 面板。
  async function loadThirdPartyPlugins(): Promise<void> {}

  return {
    loadBuiltinPlugins,
    loadThirdPartyPlugins,
  }
}
