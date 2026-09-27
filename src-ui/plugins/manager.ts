import type { Component } from 'vue'
import type { FrontendPlugin, PanelProvider, ResultItemProvider, SettingsProvider } from './types'

class PluginManager {
  private plugins: Map<string, FrontendPlugin> = new Map()
  private panelProviders: Map<string, PanelProvider> = new Map()
  private resultItemProviders: ResultItemProvider[] = []
  private settingsProviders: Map<string, SettingsProvider> = new Map()

  /** 注册一个前端插件 */
  async register(plugin: FrontendPlugin): Promise<void> {
    this.plugins.set(plugin.id, plugin)

    if (plugin.panelProvider) {
      this.panelProviders.set(plugin.panelProvider.matchType, plugin.panelProvider)
    }
    if (plugin.resultItemProvider) {
      this.resultItemProviders.push(plugin.resultItemProvider)
      // 保持同优先级时的注册顺序（稳定排序）
      this.resultItemProviders.sort((a, b) =>
        a.priority !== b.priority ? a.priority - b.priority : 0,
      )
    }
    if (plugin.settingsProvider) {
      this.settingsProviders.set(
        plugin.settingsProvider.matchComponentId,
        plugin.settingsProvider,
      )
    }

    await plugin.onInit?.()
  }

  /** 注销插件 */
  async unregister(pluginId: string): Promise<void> {
    const plugin = this.plugins.get(pluginId)
    if (!plugin) return

    await plugin.onDestroy?.()
    this.plugins.delete(pluginId)

    if (plugin.panelProvider) {
      this.panelProviders.delete(plugin.panelProvider.matchType)
    }
    if (plugin.resultItemProvider) {
      this.resultItemProviders = this.resultItemProviders.filter(
        (p) => p !== plugin.resultItemProvider,
      )
    }
    if (plugin.settingsProvider) {
      this.settingsProviders.delete(plugin.settingsProvider.matchComponentId)
    }
  }

  /** 按 panel_type 查找面板组件 */
  getPanelComponent(panelType: string): Component | null {
    return this.panelProviders.get(panelType)?.component ?? null
  }

  /** 按结果 targetType 查找自定义渲染组件（链式匹配，返回第一个命中） */
  getResultItemComponent(targetType: string): Component | null {
    for (const provider of this.resultItemProviders) {
      if (provider.matchTypes.includes(targetType)) {
        return provider.component
      }
    }
    return null
  }

  /** 按 component_id 查找自定义设置组件 */
  getSettingsComponent(componentId: string): Component | null {
    return this.settingsProviders.get(componentId)?.component ?? null
  }
}

/** 全局单例 */
export const pluginManager = new PluginManager()
