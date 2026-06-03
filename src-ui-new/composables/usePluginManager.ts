import { usePluginStore } from '@/stores/plugin-store'
import type { FrontendPlugin } from '@/plugins/types'

interface GlobEntry {
  default: FrontendPlugin
}

let builtinsLoaded = false

export function usePluginManager() {
  const pluginStore = usePluginStore()

  // 加载内置插件（幂等）。使用 import.meta.glob 自动发现 built-in/*/index.ts。
  async function loadBuiltinPlugins(): Promise<void> {
    if (builtinsLoaded) return

    const modules = import.meta.glob<GlobEntry>(
      '@/plugins/built-in/*/index.ts',
      { eager: true },
    )

    const entries = Object.entries(modules)
      .map(([path, mod]) => ({ plugin: mod.default, path }))
      .sort(
        (a, b) =>
          (a.plugin.priority ?? 100) - (b.plugin.priority ?? 100),
      )

    for (const { plugin, path } of entries) {
      try {
        await pluginStore.registerPlugin(plugin)
      } catch (err) {
        console.error(
          `[PluginManager] Failed to load built-in plugin from ${path}:`,
          err,
        )
      }
    }

    builtinsLoaded = true
  }

  return {
    loadBuiltinPlugins,
  }
}
