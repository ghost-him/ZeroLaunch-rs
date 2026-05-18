import { defineStore } from 'pinia'
import { ref } from 'vue'
import {
  configGetAllComponents, configGetSchema, configGetSettings,
  configApplySettings, configResetSettings, configSetEnabled,
  configGetActions, configExecuteAction,
} from '../bridge/commands'
import type { ComponentInfo, ComponentSchema, ConfigActionDef } from '../bridge/contract'

export const useConfigStore = defineStore('config', () => {
  const components = ref<Record<string, ComponentInfo>>({})
  const schemas = ref<Record<string, ComponentSchema>>({})
  const settings = ref<Record<string, unknown>>({})
  const isLoading = ref(false)
  const loadError = ref<string | null>(null)

  async function loadAllComponents() {
    isLoading.value = true
    loadError.value = null
    try {
      const list = await configGetAllComponents()
      const map: Record<string, ComponentInfo> = {}
      for (const c of list) {
        map[c.componentId] = c
      }
      components.value = map
    } catch (e) {
      loadError.value = String(e)
    } finally {
      isLoading.value = false
    }
  }

  async function getSchema(componentId: string): Promise<ComponentSchema> {
    if (schemas.value[componentId]) {
      return schemas.value[componentId]
    }
    const schema = await configGetSchema(componentId)
    schemas.value[componentId] = schema
    return schema
  }

  async function getSettings(componentId: string): Promise<unknown> {
    const s = await configGetSettings(componentId)
    settings.value[componentId] = s
    return s
  }

  async function applySettings(componentId: string, value: unknown) {
    await configApplySettings(componentId, value)
    settings.value[componentId] = value
  }

  async function resetSettings(componentId: string) {
    await configResetSettings(componentId)
    const s = await configGetSettings(componentId)
    settings.value[componentId] = s
  }

  async function setEnabled(componentId: string, enabled: boolean) {
    await configSetEnabled(componentId, enabled)
    if (components.value[componentId]) {
      components.value[componentId].enabled = enabled
    }
  }

  async function getActions(componentId: string): Promise<ConfigActionDef[]> {
    return configGetActions(componentId)
  }

  async function executeAction(componentId: string, action: string, params?: unknown): Promise<unknown> {
    return configExecuteAction(componentId, action, params)
  }

  return {
    components, schemas, settings, isLoading, loadError,
    loadAllComponents, getSchema, getSettings,
    applySettings, resetSettings, setEnabled,
    getActions, executeAction,
  }
})
