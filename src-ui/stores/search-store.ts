import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import {
  bridgeQuery, bridgeConfirm,
  bridgeRefreshCandidates, bridgeGetCandidatesCount,
  bridgeHideWindow,
} from '../bridge/commands'
import type { ListItem, ResultAction, BridgeQueryResponse, ConfirmResponse, PanelInteraction, PanelInteractionEvent } from '../bridge/contract'
import { onSessionReset, onPanelInteraction } from '../bridge/events'

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
  /** 当前插件面板的通用交互策略。 */
  const panelInteraction = ref<PanelInteraction | null>(null)
  /** 防抖定时器 */
  let debounceTimer: number | null = null

  // 行内参数模式
  const inlineParamState = ref<InlineParamState | null>(null)

  // 参数面板模式
  const paramPanelState = ref<ParamPanelState | null>(null)

  /** 递增序号，丢弃过期的 bridge_query 响应，避免慢请求盖写新输入。 */
  let querySeq = 0

  /** onEnter 模式确认查询在途标志：在途时忽略重复 Enter（不发查询=不加序号）。 */
  let confirmInFlight = false
  /** onEnter 模式上次确认查询的文本：同文本再次 Enter（结果已显示）→ 提示不重复按。 */
  let lastConfirmedText: string | null = null
  /** 重复 Enter 拦截提示（瞬态，SearchView 监听后弹出 notification 并复位）。 */
  const confirmBlockedHint = ref(false)
  /** 确认查询已发出提示（瞬态，仅 onEnter 模式，SearchView 监听后弹出 notification 并复位）。 */
  const confirmStartedHint = ref(false)

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

  /// 当前行内插件面板的触发词列表（来自 panel-interaction 事件）。
  /// 用于退出判定：输入不再匹配任何触发词（无空格或首词不在集合中）时立即查询退出，
  /// 退出操作独立于插件防抖配置（如从 "fy hello" 回退到 "fy" 不受防抖延迟）。
  let panelTriggerKeywords: string[] = []

  async function doQuery(raw: string, confirm = false) {
    query.value = raw
    const seq = ++querySeq

    if (raw === '') {
      results.value = []
      sessionMode.value = 'none'
      panelType.value = null
      panelData.value = null
      panelActions.value = []
      panelInteraction.value = null
      panelTriggerKeywords = []
      confirmInFlight = false
      lastConfirmedText = null
      inlineParamState.value = null
      paramPanelState.value = null
      selectedIndex.value = 0
      selectedActionIndex.value = 0
      return
    }

    // 非确认查询（输入/路由变化）：文本已变化，解除确认状态，允许后续重新确认。
    if (!confirm) {
      confirmInFlight = false
      lastConfirmedText = null
    }

    // 清空上个防抖
    if (debounceTimer !== null) {
      clearTimeout(debounceTimer)
      debounceTimer = null
    }

    // 退出判定（优先于防抖）：行内插件面板内，输入不再匹配当前插件触发词
    // （无空格或首词不在触发词集合）→ 立即查询退出，不受插件防抖延迟。
    // 判定结果仍由后端路由裁决，正确性无损。
    const isInPanel = panelTriggerKeywords.length > 0
    const firstWord = raw.split(' ')[0]
    const shouldExit = isInPanel && (!raw.includes(' ') || !panelTriggerKeywords.includes(firstWord))
    // 防抖：未达间隔前不发送 IPC（首次或 dm=0 时直发；onEnter 手动模式忽略防抖）
    const dm = shouldExit
      ? 0
      : (panelInteraction.value?.queryTrigger === 'onEnter'
        ? 0
        : (panelInteraction.value?.queryDebounceMs ?? 0))
    if (dm > 0) {
      debounceTimer = window.setTimeout(() => {
        debounceTimer = null
        doQueryImpl(raw, seq, confirm)
      }, dm)
      return
    }

    doQueryImpl(raw, seq, confirm)
  }

  async function doQueryImpl(raw: string, seq: number, confirm: boolean) {
    try {
      console.log(`[doQuery] Sending query: "${raw}" (seq=${seq})`)
      const resp: BridgeQueryResponse = await bridgeQuery(raw, confirm)

      if (seq !== querySeq) return

      // 确认查询响应已到达（无论结果如何），解除在途标志，允许下一次确认。
      confirmInFlight = false

      selectedActionIndex.value = 0

      switch (resp.mode) {
        case 'search':
          results.value = resp.results
          panelInteraction.value = null
          panelTriggerKeywords = []
          sessionMode.value = 'search'
          selectedIndex.value = 0
          break
        case 'empty':
          results.value = []
          panelInteraction.value = null
          panelTriggerKeywords = []
          sessionMode.value = 'search'
          selectedIndex.value = 0
          break
        case 'inline_param':
          results.value = []
          panelInteraction.value = null
          panelTriggerKeywords = []
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
      if (seq !== querySeq) return
      // 查询失败同样解除在途标志，避免后续 Enter 被永久拦截。
      confirmInFlight = false
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
    // 手动查询模式（queryTrigger=onEnter）：Enter 触发一次确认查询（confirm=true），
    // 后端据此直接执行面板动作（如翻译插件 on_enter 模式翻译）。
    if (panelInteraction.value?.queryTrigger === 'onEnter') {
      // 确认查询在途：忽略重复 Enter（不发查询=不加序号），首次结果返回后自然显示。
      if (confirmInFlight) return
      // 同文本已确认过（结果已显示）：提示不重复按，避免重复 LLM 调用。
      if (lastConfirmedText === query.value) {
        confirmBlockedHint.value = true
        return
      }
      confirmInFlight = true
      lastConfirmedText = query.value
      confirmStartedHint.value = true
      void doQuery(query.value, true)
      return
    }
    // 自动查询模式（onInput）：Enter 执行面板默认动作。
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

  function resetLocalSession() {
    // 取消未触发的防抖定时器
    if (debounceTimer !== null) {
      clearTimeout(debounceTimer)
      debounceTimer = null
    }
    // 递增序号使所有在途响应的 seq 失效，防止慢请求盖写新状态
    querySeq++
    query.value = ''
    results.value = []
    sessionMode.value = 'none'
    panelType.value = null
    panelData.value = null
    panelActions.value = []
    panelInteraction.value = null
    panelTriggerKeywords = []
    confirmInFlight = false
    lastConfirmedText = null
    inlineParamState.value = null
    paramPanelState.value = null
    selectedIndex.value = 0
    selectedActionIndex.value = 0
  }

  function resetSessionAndHide() {
    resetLocalSession()
    hideWindow()
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

  // 监听后端 session-reset 事件，同步前端状态
  onSessionReset(() => {
    resetLocalSession()
  })

  // 监听后端推送的面板交互策略：路由确定插件面板时推送一次，面板内不再重复。
  // 事件总是描述当前会话最新路由的面板，无条件接受即可；退出面板由各响应分支清空。
  onPanelInteraction((payload: PanelInteractionEvent) => {
    panelInteraction.value = payload.interaction
    panelTriggerKeywords = payload.triggerKeywords ?? []
  })

  return {
    query, results, selectedIndex, selectedActionIndex, sessionMode, cachedCount,
    panelType, panelData, panelActions, panelInteraction,
    confirmBlockedHint,
    confirmStartedHint,
    inlineParamState, paramPanelState,
    isIdle, selectedItem,
    doQuery, doConfirm, selectNext, selectPrev,
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
