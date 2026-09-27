import { defineStore } from 'pinia'
import type { Component } from 'vue'
import type { FrontendPlugin } from '@/plugins/types'
import { pluginManager } from '@/plugins/manager'

export const usePluginStore = defineStore('plugin', () => {
  async function registerPlugin(plugin: FrontendPlugin) {
    await pluginManager.register(plugin)
  }

  async function unregisterPlugin(pluginId: string) {
    await pluginManager.unregister(pluginId)
  }

  function getPanelComponent(panelType: string): Component | null {
    return pluginManager.getPanelComponent(panelType)
  }

  function getResultItemComponent(targetType: string): Component | null {
    return pluginManager.getResultItemComponent(targetType)
  }

  function getSettingsComponent(componentId: string): Component | null {
    return pluginManager.getSettingsComponent(componentId)
  }

  return {
    registerPlugin,
    unregisterPlugin,
    getPanelComponent,
    getResultItemComponent,
    getSettingsComponent,
  }
})
