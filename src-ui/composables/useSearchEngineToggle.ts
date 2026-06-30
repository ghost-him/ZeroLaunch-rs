import { useMessage } from 'naive-ui'
import { useConfigStore } from '../stores/config-store'
import type { ComponentInfo } from '../bridge/contract'

export function useSearchEngineToggle(getEngines: () => ComponentInfo[]) {
  const configStore = useConfigStore()
  const message = useMessage()

  async function onToggle(componentId: string, val: boolean) {
    const engines = getEngines()

    if (!val) {
      const enabledEngines = engines.filter(e => e.enabled)
      if (enabledEngines.length <= 1 && enabledEngines[0]?.componentId === componentId) {
        message.warning('必须至少保持一个检索引擎处于开启状态')
        return
      }

      try {
        await configStore.setEnabled(componentId, false)
      } catch (e) {
        console.error(e)
      }
      return
    }

    try {
      for (const engine of engines) {
        if (engine.componentId !== componentId && engine.enabled) {
          await configStore.setEnabled(engine.componentId, false)
        }
      }
      await configStore.setEnabled(componentId, true)
    } catch (e) {
      console.error(e)
    }
  }

  return { onToggle }
}
