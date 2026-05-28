import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import {
  bridgeQuery, bridgeConfirm, bridgeWake, bridgeReset,
  bridgeRefreshCandidates, bridgeGetCandidatesCount,
  bridgeHideWindow,
} from '../bridge/commands'
import type { ListItem, ResultAction, BridgeQueryResponse, ConfirmResponse } from '../bridge/contract'

export type SessionMode = 'none' | 'search' | 'inline_param' | 'param_panel' | 'inline_plugin' | 'full_page_plugin'

export interface InlineParamState {
  candidateId: number
  triggerKeyword: string
  paramInput: string
  userArgCount: number
}

export interface ParamField {
  index: number
  label: string
  value: string
}

export interface ParamPanelState {
  candidateId: number
  candidateItem: ListItem
  fields: ParamField[]
  focusedFieldIndex: number
}

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

  // 行内参数模式
  const inlineParamState = ref<InlineParamState | null>(null)

  // 参数面板模式
  const paramPanelState = ref<ParamPanelState | null>(null)

  // ---- 派生 ----
  const isIdle = computed(() => query.value === '')

  const selectedItem = computed(() => {
    if (results.value.length === 0) return null
    const idx = Math.min(selectedIndex.value, results.value.length - 1)
    return results.value[idx]
  })

  // ---- 转义序列解析 ----

  /**
   * 解析行内参数输入，支持转义：
   * - 未转义空格 = 参数分隔符
   * - \空格 = 字面空格
   * - \\ = 字面反斜杠
   */
  function parseInlineArgs(input: string): string[] {
    const args: string[] = []
    let current = ''
    let i = 0

    while (i < input.length) {
      if (input[i] === '\\' && i + 1 < input.length) {
        if (input[i + 1] === ' ') {
          current += ' '
          i += 2
        } else if (input[i + 1] === '\\') {
          current += '\\'
          i += 2
        } else {
          current += input[i]
          i++
        }
      } else if (input[i] === ' ') {
        if (current.length > 0) {
          args.push(current)
          current = ''
        }
        i++
      } else {
        current += input[i]
        i++
      }
    }
    if (current.length > 0) {
      args.push(current)
    }
    return args
  }

  // ---- 动作 ----

  async function doQuery(raw: string) {
    query.value = raw

    if (raw === '') {
      results.value = []
      sessionMode.value = 'none'
      panelType.value = null
      inlineParamState.value = null
      paramPanelState.value = null
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
        case 'inline_param':
          results.value = []
          sessionMode.value = 'inline_param'
          inlineParamState.value = {
            candidateId: resp.inlineParam.candidateId,
            triggerKeyword: resp.inlineParam.triggerKeyword,
            paramInput: '',
            userArgCount: resp.inlineParam.userArgCount,
          }
          query.value = ''
          break
        case 'plugin_panel':
        case 'plugin_immersive':
          results.value = []
          sessionMode.value = resp.mode === 'plugin_panel' ? 'inline_plugin' : 'full_page_plugin'
          panelType.value = resp.panelType
          panelData.value = resp.panelData
          panelActions.value = resp.panelActions
          selectedIndex.value = 0
          break
      }
    } catch (e) {
      console.error('[doQuery] Query failed:', e)
    }
  }

  async function doConfirm(index?: number, actionId?: string) {
    // 插件模式（行内或全页面）
    if (sessionMode.value === 'inline_plugin' || sessionMode.value === 'full_page_plugin') {
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

      panelType.value = null
      resetSessionAndHide()
      return
    }

    // Search mode
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

    let resp: ConfirmResponse
    try {
      resp = await bridgeConfirm({
        candidateId: item.id,
        actionId: targetActionId,
        queryText: query.value,
      })
    } catch (e) {
      console.error('[doConfirm] Search action failed:', e)
      return
    }

    // 后端判定需要参数面板
    if (resp.status === 'enterParamPanel') {
      const fields: ParamField[] = Array.from({ length: resp.userArgCount }, (_, i) => ({
        index: i,
        label: `参数 ${i + 1}`,
        value: '',
      }))
      sessionMode.value = 'param_panel'
      paramPanelState.value = {
        candidateId: resp.candidateId,
        candidateItem: item,
        fields,
        focusedFieldIndex: 0,
      }
      return
    }

    // status === 'executed'
    resetSessionAndHide()
  }

  // ---- 行内参数模式 ----

  /// 退出行内参数模式（纯前端清理，后端模式由下一次 bridge_query 自然重置）。
  /// 有触发关键词时恢复搜索；无关键词时调用 bridge_query("") 通知后端重置。
  function exitInlineParamMode() {
    const kw = inlineParamState.value?.triggerKeyword ?? ''
    inlineParamState.value = null

    if (kw) {
      query.value = kw
      doQuery(kw)
    } else {
      doQuery('')
    }
  }

  async function confirmInlineParam() {
    if (!inlineParamState.value) return

    const { candidateId, paramInput, userArgCount } = inlineParamState.value
    const args = parseInlineArgs(paramInput)

    if (args.length < userArgCount) {
      console.warn(`需要 ${userArgCount} 个参数，实际输入 ${args.length} 个`)
      return
    }

    try {
      await bridgeConfirm({
        candidateId,
        actionId: 'execute',
        queryText: inlineParamState.value.triggerKeyword,
        userArgs: args,
      })
    } catch (e) {
      console.error('[confirmInlineParam] failed:', e)
      return
    }

    inlineParamState.value = null
    resetSessionAndHide()
  }

  // ---- 参数面板模式 ----

  /// 搜索模式下按 Enter：统一走 bridge_confirm。
  /// 后端自行判断是执行还是进入参数面板，前端根据响应渲染。
  function handleEnterInSearchMode() {
    doConfirm()
  }

  /// 退出参数面板模式（纯前端清理）。
  /// 后端模式由下一次 bridge_query 自然重置。
  function exitParamPanelMode() {
    paramPanelState.value = null
    doQuery('')
  }

  async function confirmParamPanel() {
    if (!paramPanelState.value) return

    const { candidateId, fields } = paramPanelState.value
    const userArgs = fields.map((f) => f.value)

    if (userArgs.some((arg) => arg.trim() === '')) {
      return
    }

    try {
      await bridgeConfirm({
        candidateId,
        actionId: 'execute',
        queryText: query.value,
        userArgs,
      })
    } catch (e) {
      console.error('[confirmParamPanel] failed:', e)
      return
    }

    paramPanelState.value = null
    resetSessionAndHide()
  }

  function paramPanelFocusNext() {
    if (!paramPanelState.value) return
    const { fields, focusedFieldIndex } = paramPanelState.value
    paramPanelState.value = {
      ...paramPanelState.value,
      focusedFieldIndex: Math.min(focusedFieldIndex + 1, fields.length - 1),
    }
  }

  function paramPanelFocusPrev() {
    if (!paramPanelState.value) return
    paramPanelState.value = {
      ...paramPanelState.value,
      focusedFieldIndex: Math.max(paramPanelState.value.focusedFieldIndex - 1, 0),
    }
  }

  // ---- 插件模式 ----

  /// 退出行内插件模式（纯前端清理）。
  /// 后端模式由下一次 bridge_query 自然重置。
  function exitPluginMode() {
    panelType.value = null
    panelData.value = null
    panelActions.value = []
    doQuery('')
  }

  function confirmPluginAction() {
    doConfirm()
  }

  /// 退出全页面插件模式（纯前端清理）。
  function exitFullPagePlugin() {
    panelType.value = null
    panelData.value = null
    doQuery('')
  }

  // ---- 会话管理 ----

  function hideWindow() {
    bridgeHideWindow().catch((e) => console.warn('[hideWindow] Failed to hide window:', e))
  }

  function resetSessionAndHide() {
    query.value = ''
    results.value = []
    sessionMode.value = 'none'
    hideWindow()
  }

  async function doWake() {
    await bridgeWake()
    query.value = ''
    results.value = []
    sessionMode.value = 'none'
    panelType.value = null
    inlineParamState.value = null
    paramPanelState.value = null
  }

  function doReset() {
    bridgeReset()
    query.value = ''
    results.value = []
    sessionMode.value = 'none'
    panelType.value = null
    inlineParamState.value = null
    paramPanelState.value = null
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
    panelType, panelData, panelActions,
    inlineParamState, paramPanelState,
    isIdle, selectedItem,
    doQuery, doConfirm, doWake, doReset, selectNext, selectPrev,
    refreshCandidates, fetchCandidatesCount, hideWindow,
    // 行内参数模式
    exitInlineParamMode, confirmInlineParam,
    // 参数面板模式
    handleEnterInSearchMode, exitParamPanelMode,
    confirmParamPanel, paramPanelFocusNext, paramPanelFocusPrev,
    // 插件模式
    exitPluginMode, confirmPluginAction, exitFullPagePlugin,
  }
})
