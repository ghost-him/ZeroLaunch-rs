import { usePluginStore } from '@/stores/plugin-store'
import type { FrontendPlugin } from '@/plugins/types'
import { pluginList, pluginGetManifest } from '@/bridge/commands'
import ThirdPartyPanelHost from '@/plugins/third-party-host/ThirdPartyPanelHost.vue'
import ThirdPartySettingsHost from '@/plugins/third-party-host/ThirdPartySettingsHost.vue'
import { h } from 'vue'

interface GlobEntry {
  default: FrontendPlugin
}

let builtinsLoaded = false
let thirdPartyLoaded = false

export function usePluginManager() {
  const pluginStore = usePluginStore()

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

  async function registerThirdPartyPlugin(
    info: { pluginId: string; name: string; version: string },
    manifest: Record<string, unknown>,
  ): Promise<void> {
    const ui = manifest?.ui as Record<string, unknown> | undefined
    if (!ui) return

    const pluginId = info.pluginId

    if (ui.panelEntry) {
      const panelEntryUrl = `zlplugin://${pluginId}/ui/${ui.panelEntry}`
      const wrapper = {
        setup(props: { data: unknown; actions: unknown[] }) {
          return () =>
            h(ThirdPartyPanelHost, {
              pluginId,
              panelEntryUrl,
              data: props.data,
              actions: props.actions as never[],
            })
        },
      }
      await pluginStore.registerPlugin({
        id: `third-party-${pluginId}-panel`,
        name: `${info.name} Panel`,
        version: info.version,
        description: '',
        panelProvider: {
          matchType: `third-party:${pluginId}`,
          component: wrapper as never,
        },
      })
    }

    if (ui.settingsEntry) {
      const settingsEntryUrl = `zlplugin://${pluginId}/ui/${ui.settingsEntry}`
      const wrapper = {
        setup(
          props: { currentSettings: unknown },
          { emit }: { emit: (e: string, v: unknown) => void },
        ) {
          return () =>
            h(ThirdPartySettingsHost, {
              pluginId,
              settingsEntryUrl,
              currentSettings: props.currentSettings,
              onSave: (s: unknown) => emit('save', s),
            })
        },
      }
      await pluginStore.registerPlugin({
        id: `third-party-${pluginId}-settings`,
        name: `${info.name} Settings`,
        version: info.version,
        description: '',
        settingsProvider: {
          matchComponentId: pluginId,
          component: wrapper as never,
        },
      })
    }
  }

  async function loadThirdPartyPlugins(): Promise<void> {
    if (thirdPartyLoaded) return

    try {
      const installed = await pluginList()
      for (const info of installed) {
        try {
          const manifest = await pluginGetManifest(info.pluginId) as Record<string, unknown>
          await registerThirdPartyPlugin(info, manifest)
        } catch (err) {
          console.error(
            `[PluginManager] Failed to register third-party plugin ${info.pluginId}:`,
            err,
          )
        }
      }
    } catch (err) {
      console.error('[PluginManager] Failed to list third-party plugins:', err)
    }

    thirdPartyLoaded = true
  }

  return {
    loadBuiltinPlugins,
    loadThirdPartyPlugins,
  }
}
