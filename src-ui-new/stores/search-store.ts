import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { bridgeQuery, bridgeConfirm, bridgeWake, bridgeReset, bridgeRefreshCandidates, bridgeGetCandidatesCount } from '../bridge/commands'
import type { ListItem, ResultAction, BridgeQueryResponse } from '../bridge/contract'

export type SessionMode = 'none' | 'search' | 'plugin'

export const useSearchStore = defineStore('search', () => {
  // ---- 状态 ----
  const query = ref('')
  const results = ref<ListItem[]>([])
  const selectedIndex = ref(0)
  const selectedActionIndex = ref(0)
  const sessionMode = ref<SessionMode>('none')
  const cachedCount = ref(0)

  // 插件面板
  const panelType = ref<string | null>(null)
  const panelData = ref<unknown>(null)
  const panelActions = ref<ResultAction[]>([])
  const keepSearchBar = ref(true)

  // ---- 派生 ----
  const isIdle = computed(() => query.value === '')

  const selectedItem = computed(() => {
    if (results.value.length === 0) return null
    const idx = Math.min(selectedIndex.value, results.value.length - 1)
    return results.value[idx]
  })

  // ---- 动作 ----
  async function doQuery(raw: string) {
    query.value = raw

    if (raw === '') {
      results.value = []
      sessionMode.value = 'none'
      panelType.value = null
      selectedIndex.value = 0
      selectedActionIndex.value = 0
      return
    }

    try {
      const resp: BridgeQueryResponse = await bridgeQuery(raw)

      selectedActionIndex.value = 0

      switch (resp.mode) {
        case 'search':
          results.value = resp.results
          sessionMode.value = 'search'
          selectedIndex.value = 0
          break
        case 'empty':
          results.value = []
          sessionMode.value = 'search'
          selectedIndex.value = 0
          break
        case 'plugin_panel':
        case 'plugin_immersive':
          results.value = []
          sessionMode.value = 'plugin'
          panelType.value = resp.panelType
          panelData.value = resp.panelData
          panelActions.value = resp.panelActions
          keepSearchBar.value = resp.mode === 'plugin_panel'
          selectedIndex.value = 0
          break
      }
    } catch (e) {
      console.error('[doQuery] Query failed:', e)
    }
  }

  async function doConfirm(index?: number, actionId?: string) {
    // 插件模式：使用虚拟 candidate_id=0（插件模式下后端按 plugin_id 路由，不依赖 candidate_id）
    if (sessionMode.value === 'plugin') {
      let targetActionId = actionId
      if (!targetActionId) {
        const action = panelActions.value[selectedActionIndex.value]
        targetActionId = action?.id ?? panelActions.value.find((a) => a.isDefault)?.id
      }
      if (!targetActionId) return

      try {
        await bridgeConfirm({
          candidateId: 0,
          actionId: targetActionId,
          queryText: query.value,
        })
      } catch (e) {
        console.error('[doConfirm] Plugin action failed:', e)
        return
      }

      query.value = ''
      results.value = []
      sessionMode.value = 'none'
      panelType.value = null
      return
    }

    const idx = index ?? selectedIndex.value
    const item = results.value[idx]
    if (!item) return

    let targetActionId = actionId
    if (!targetActionId) {
      const actionIdx = Math.min(selectedActionIndex.value, item.actions.length - 1)
      const action = item.actions[actionIdx]
      targetActionId = action?.id ?? item.actions.find((a) => a.isDefault)?.id
    }
    if (!targetActionId) return

    try {
      await bridgeConfirm({
        candidateId: item.id,
        actionId: targetActionId,
        queryText: query.value,
      })
    } catch (e) {
      console.error('[doConfirm] Search action failed:', e)
      return
    }

    // 执行成功后才清空输入
    query.value = ''
    results.value = []
    sessionMode.value = 'none'
  }

  async function doWake() {
    await bridgeWake()
    query.value = ''
    results.value = []
    sessionMode.value = 'none'
    panelType.value = null
  }

  function doReset() {
    bridgeReset()
    query.value = ''
    results.value = []
    sessionMode.value = 'none'
    panelType.value = null
    selectedIndex.value = 0
  }

  function selectNext() {
    if (results.value.length === 0) return
    selectedIndex.value = Math.min(selectedIndex.value + 1, results.value.length - 1)
  }

  function selectPrev() {
    selectedIndex.value = Math.max(selectedIndex.value - 1, 0)
  }

  async function refreshCandidates(): Promise<number> {
    cachedCount.value = await bridgeRefreshCandidates()
    return cachedCount.value
  }

  async function fetchCandidatesCount() {
    cachedCount.value = await bridgeGetCandidatesCount()
  }

  return {
    query, results, selectedIndex, selectedActionIndex, sessionMode, cachedCount,
    panelType, panelData, panelActions, keepSearchBar,
    isIdle, selectedItem,
    doQuery, doConfirm, doWake, doReset, selectNext, selectPrev,
    refreshCandidates, fetchCandidatesCount,
  }
})
