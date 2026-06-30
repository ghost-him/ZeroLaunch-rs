import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { Component } from 'vue'
import type { FrontendPlugin } from '@/plugins/types'
import { pluginManager } from '@/plugins/manager'
import type { ListItem, ResultAction } from '@/bridge/contract'

export const usePluginStore = defineStore('plugin', () => {
  const loadedPlugins = ref<FrontendPlugin[]>([])

  function syncPluginList() {
    loadedPlugins.value = pluginManager.getLoadedPlugins()
  }

  async function registerPlugin(plugin: FrontendPlugin) {
    await pluginManager.register(plugin)
    syncPluginList()
  }

  async function unregisterPlugin(pluginId: string) {
    await pluginManager.unregister(pluginId)
    syncPluginList()
  }

  function getPanelComponent(panelType: string): Component | null {
    return pluginManager.getPanelComponent(panelType)
  }

  function getResultItemComponent(targetType: string): Component | null {
    return pluginManager.getResultItemComponent(targetType)
  }

  function getExtraActions(item: ListItem, targetType: string): ResultAction[] {
    return pluginManager.getExtraActions(item, targetType)
  }

  function getSettingsComponent(componentId: string): Component | null {
    return pluginManager.getSettingsComponent(componentId)
  }

  return {
    loadedPlugins,
    registerPlugin,
    unregisterPlugin,
    getPanelComponent,
    getResultItemComponent,
    getExtraActions,
    getSettingsComponent,
  }
})
