import { usePluginStore } from '@/stores/plugin-store'
import type { FrontendPlugin } from '@/plugins/types'
import { pluginList, pluginGetManifest } from '@/bridge/commands'
import { onPluginInstalled, onPluginUninstalled } from '@/bridge/events'
import ThirdPartyPanelHost from '@/plugins/third-party-host/ThirdPartyPanelHost.vue'
import ThirdPartySettingsHost from '@/plugins/third-party-host/ThirdPartySettingsHost.vue'
import type { ResultAction } from '@/bridge/contract'
import { defineComponent, h } from 'vue'

interface GlobEntry {
  default: FrontendPlugin
}

let builtinsLoaded = false
let thirdPartyLoaded = false
let eventsRegistered = false
/// Tracks already-registered third-party plugin IDs to prevent duplicate
/// registrations on reload or duplicate events.
const registeredThirdPartyIds = new Set<string>()

/**
 * 构造 WebView 可加载的第三方插件资源 URL。
 * 拼上插件版本：升级后 URL 变化，ESM Module Map 不再按旧 URL 返回缓存模块。
 */
function buildPluginAssetUrl(pluginId: string, entry: string, versionKey: string): string {
  return `http://zlplugin.localhost/${pluginId}/${entry.replace(/^\/+/, '')}?v=${encodeURIComponent(versionKey)}`
}

export function usePluginManager() {
  const pluginStore = usePluginStore()

  async function loadBuiltinPlugins(): Promise<void> {
    if (builtinsLoaded) return

    const modules = import.meta.glob<GlobEntry>(
      '/src-ui/plugins/built-in/*/index.ts',
      { eager: true },
    )

    const entries = Object.entries(modules)
      // 排除 _template 模板目录
      .filter(([path]) => !path.includes('/_template/'))
      .map(([path, mod]) => ({ plugin: mod.default, path }))
      .sort(
        (a, b) =>
          (a.plugin.priority ?? 50) - (b.plugin.priority ?? 50),
      )

    for (const { plugin, path } of entries) {
      try {
        pluginStore.registerPlugin(plugin)
      } catch (err) {
        console.error(`[PluginManager] 注册内置插件失败: ${path}`, err)
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
    // E-3：URL 版本键用安装时间戳（每次注册唯一，重装/覆盖必然变化），
    // 不用开发者可控的 manifest version——避免同版本重装仍命中 ESM 缓存。
    const installStamp = Date.now().toString(36)

    // 去重：仅跳过「本次会话内已注册且未卸载」的重复注册（启动加载路径）。
    // 事件路径（安装/重装/覆盖）在调用前已 unregister，此处不拦。
    if (registeredThirdPartyIds.has(pluginId)) return
    registeredThirdPartyIds.add(pluginId)

    if (ui.panelEntry) {
      const panelEntryUrl = buildPluginAssetUrl(pluginId, ui.panelEntry as string, installStamp)
      // 显式 defineComponent + props/emits 声明：不声明则监听器走 attrs 透传
      // 与 emit 双路径，导致 save 双触发（IPC×2/toast×2）。
      const wrapper = defineComponent({
        props: {
          data: { type: Object as () => unknown, required: true },
          actions: { type: Array as () => ResultAction[], required: true },
        },
        setup(props) {
          return () =>
            h(ThirdPartyPanelHost, {
              pluginId,
              panelEntryUrl,
              data: props.data,
              actions: props.actions,
            })
        },
      })
      await pluginStore.registerPlugin({
        id: `third-party-${pluginId}-panel`,
        name: `${info.name} Panel`,
        version: info.version,
        description: '',
        panelProvider: {
          matchType: `third-party:${pluginId}`,
          component: wrapper,
        },
      })
    }

    if (ui.settingsEntry) {
      const settingsEntryUrl = buildPluginAssetUrl(pluginId, ui.settingsEntry as string, installStamp)
      // 同 panel wrapper：显式声明 emits 避免 save 双触发
      const wrapper = defineComponent({
        props: {
          currentSettings: { type: Object as () => unknown, default: undefined },
        },
        emits: ['save'],
        setup(props, { emit }) {
          return () =>
            h(ThirdPartySettingsHost, {
              pluginId,
              settingsEntryUrl,
              currentSettings: props.currentSettings,
              onSave: (s: unknown) => emit('save', s),
            })
        },
      })
      await pluginStore.registerPlugin({
        id: `third-party-${pluginId}-settings`,
        name: `${info.name} Settings`,
        version: info.version,
        description: '',
        settingsProvider: {
          matchComponentId: pluginId,
          component: wrapper,
        },
      })
    }
  }

  /// Unregister all frontend providers for a third-party plugin.
  async function unregisterThirdPartyPlugin(pluginId: string): Promise<void> {
    registeredThirdPartyIds.delete(pluginId)
    await pluginStore.unregisterPlugin(`third-party-${pluginId}-panel`)
    await pluginStore.unregisterPlugin(`third-party-${pluginId}-settings`)
  }

  /// Register event listeners for runtime plugin install / uninstall.
  function setupEventListeners(): void {
    if (eventsRegistered) return
    eventsRegistered = true

    onPluginInstalled(async (payload) => {
      try {
        // 覆盖安装/重装：先卸载旧注册（清 ESM Module Map 缓存）再注册新 URL，
        // 否则同版本重装 URL 不变仍命中旧模块（E-3）。
        await unregisterThirdPartyPlugin(payload.pluginId)
        const manifest = await pluginGetManifest(payload.pluginId) as Record<string, unknown>
        await registerThirdPartyPlugin(
          {
            pluginId: payload.pluginId,
            name: payload.name ?? '',
            version: payload.version ?? '',
          },
          manifest,
        )
      } catch (err) {
        console.error(
          `[PluginManager] Failed to register new plugin ${payload.pluginId}:`,
          err,
        )
      }
    })

    onPluginUninstalled(async (payload) => {
      try {
        await unregisterThirdPartyPlugin(payload.pluginId)
      } catch (err) {
        console.error(
          `[PluginManager] Failed to unregister plugin ${payload.pluginId}:`,
          err,
        )
      }
    })
  }

  async function loadThirdPartyPlugins(): Promise<void> {
    if (thirdPartyLoaded) return

    // Set up event listeners for dynamic plugin install/uninstall
    setupEventListeners()

    try {
      const installed = await pluginList()
      for (const info of installed) {
        // 内置插件无 manifest 文件，跳过第三方注册流程
        if (info.kind === 'builtin') continue
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
