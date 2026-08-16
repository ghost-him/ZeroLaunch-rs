import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import {
  bridgeQuery, bridgeConfirm,
  bridgeRefreshCandidates, bridgeGetCandidatesCount,
  bridgeHideWindow,
} from '../bridge/commands'
import type { ListItem, ResultAction, BridgeQueryResponse, ConfirmResponse, PanelInteraction, SessionStateEvent } from '../bridge/contract'
import { onSessionState } from '../bridge/events'

/**
 * 会话模式 —— 与后端响应 `BridgeQueryResponse.mode` 同词表（snake_case）。
 * 注意：与事件 `session-state.presentation`（camelCase，见 contract.ts PresentationMode）
 * 是不同值空间，勿混用；两者 'none'/'search' 拼写恰好相同。
 * 插件面板形态直接透传响应 mode：'plugin_panel'（行内）/ 'plugin_immersive'（全页面）。
 */
export type SessionMode =
  | 'none'
  | 'search'
  | 'inline_param'
  | 'param_panel'
  | 'plugin_panel'
  | 'plugin_immersive'

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
  /** 当前插件面板所属插件 ID（来自 session-state 事件；宿主面板为 null）。供键盘解释器转发面板按键动作。 */
  const currentPluginId = ref<string | null>(null)
  /** 插件元数据缓存（pluginId → 显示名/图标/形态）：由 useKeyboardRouter 在插件列表刷新时填充，
   *  供 Footer/搜索栏前缀渲染当前插件标识。 */
  const pluginMeta = ref<Record<string, { name: string; icon: string | null; mode: 'inline' | 'panel' }>>({})
  /** 会话代际：随 bridge_query / bridge_confirm 响应单调递增更新，确认请求回传校验。 */
  const currentGeneration = ref(0)
  /** 防抖定时器 */
  let debounceTimer: number | null = null

  /// 取消挂起中的防抖查询。输入清空、退出面板、隐藏窗口等「放弃输入」的路径必须调用，
  /// 否则定时器到期仍会发出 IPC，造成后端真实执行查询（如消耗 LLM 配额的幽灵翻译）。
  function cancelPendingDebounce() {
    if (debounceTimer !== null) {
      clearTimeout(debounceTimer)
      debounceTimer = null
    }
  }

  // 行内参数模式
  const inlineParamState = ref<InlineParamState | null>(null)

  // 参数面板模式
  const paramPanelState = ref<ParamPanelState | null>(null)

  /** 递增序号，丢弃过期的 bridge_query 响应，避免慢请求盖写新输入。 */
  let querySeq = 0

  /** 确认查询在途标志：在途时忽略重复 Enter（不发查询=不加序号）。 */
  const confirmInFlight = ref(false)
  /** 面板查询在途标志：查询已发出且仍属于当前插件面板（未退出），响应到达后清除。供插件面板感知「查询处理中」。 */
  const panelQueryInFlight = ref(false)

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

  /// 当前行内插件面板的触发词列表（来自 session-state 事件）。
  /// 用于退出判定：输入不再匹配任何触发词（无空格或首词不在集合中）时立即查询退出，
  /// 退出操作独立于插件防抖配置（如从 "fy hello" 回退到 "fy" 不受防抖延迟）。
  let panelTriggerKeywords: string[] = []

  /// 查询文本是否仍属于当前插件面板：首词为空格分隔的触发词（镜像后端 SessionDispatcher::match_trigger）。
  /// 输入交互层判定（RULES.md 前后端职责边界）：仅用于 IPC 前时序决策（防抖豁免、在途提示），
  /// 权威路由仍由后端 route_query 裁决；判定参数（触发词）来自后端 session-state 事件，
  /// 镜像变更须与后端同步（frontend-input-interaction 规则）。
  function queryStillInPanel(raw: string): boolean {
    if (panelTriggerKeywords.length === 0) return false
    const firstWord = raw.split(' ')[0]
    return raw.includes(' ') && panelTriggerKeywords.includes(firstWord)
  }

  async function doQuery(raw: string, confirm = false) {
    query.value = raw
    const seq = ++querySeq

    // 任何新查询（含清空/退出）都取代挂起的防抖定时器
    cancelPendingDebounce()

    if (raw === '') {
      results.value = []
      sessionMode.value = 'none'
      panelType.value = null
      panelData.value = null
      panelActions.value = []
      panelInteraction.value = null
      panelTriggerKeywords = []
      currentPluginId.value = null
      confirmInFlight.value = false
      inlineParamState.value = null
      paramPanelState.value = null
      selectedIndex.value = 0
      selectedActionIndex.value = 0
      panelQueryInFlight.value = false
      return
    }

    // 非确认查询（输入/路由变化）：解除确认在途状态（文本已变化，之后 Enter 可重新确认）。
    if (!confirm) {
      confirmInFlight.value = false
    }

    // 退出判定（优先于防抖）：行内插件面板内，输入不再匹配当前插件触发词
    // （无空格或首词不在触发词集合）→ 立即查询退出，不受插件防抖延迟。
    // 判定结果仍由后端路由裁决，正确性无损。
    const shouldExit = panelTriggerKeywords.length > 0 && !queryStillInPanel(raw)
    // 防抖：未达间隔前不发送 IPC（首次或 dm=0 时直发；onEnter 手动模式忽略防抖）
    const dm = shouldExit
      ? 0
      : (panelInteraction.value?.queryTrigger === 'onEnter'
        ? 0
        : (panelInteraction.value?.queryDebounceMs ?? 0))
    if (dm > 0) {
      debounceTimer = window.setTimeout(() => {
        debounceTimer = null
        // 兜底：期间已有新查询（seq 已递增）则不再发 IPC
        if (seq !== querySeq) return
        doQueryImpl(raw, seq, confirm)
      }, dm)
      return
    }

    doQueryImpl(raw, seq, confirm)
  }

  async function doQueryImpl(raw: string, seq: number, confirm: boolean) {
    // 面板查询在途（通用状态）：查询仍属于当前插件面板时置位，响应到达后清除。
    // 供插件面板感知「查询处理中」（如翻译面板据此提示「已开始翻译」）。
    panelQueryInFlight.value = queryStillInPanel(raw)
    try {
      console.log(`[doQuery] Sending query: "${raw}" (seq=${seq})`)
      const resp: BridgeQueryResponse = await bridgeQuery(raw, confirm)

      if (seq !== querySeq) return

      // 确认查询响应已到达（无论结果如何），解除在途标志，允许下一次确认。
      confirmInFlight.value = false

      selectedActionIndex.value = 0

      switch (resp.mode) {
        case 'search':
          results.value = resp.results
          panelInteraction.value = null
          panelTriggerKeywords = []
          currentPluginId.value = null
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
          // 直接透传响应 mode（与 store SessionMode 词表一致，snake_case）
          sessionMode.value = resp.mode
          panelType.value = resp.panelType
          panelData.value = resp.panelData
          panelActions.value = resp.panelActions
          selectedIndex.value = 0
          break
      }

      // 所有分支同步最新会话代际（单调递增，不倒退）
      if (resp.generation >= currentGeneration.value) {
        currentGeneration.value = resp.generation
      }
    } catch (e) {
      if (seq !== querySeq) return
      // 查询失败同样解除在途标志，避免后续 Enter 被永久拦截。
      confirmInFlight.value = false
      console.error('[doQuery] Query failed:', e)
    } finally {
      // 仅当前查询可清除在途标志：过期响应不得覆盖新查询的在途状态。
      if (seq === querySeq) {
        panelQueryInFlight.value = false
      }
    }
  }

  async function doConfirm(index?: number, actionId?: string) {
    // 插件模式（行内或全页面）
    if (sessionMode.value === 'plugin_panel' || sessionMode.value === 'plugin_immersive') {
      let targetActionId = actionId
      if (!targetActionId) {
        const action = panelActions.value[selectedActionIndex.value]
        targetActionId = action?.id ?? panelActions.value.find((a) => a.isDefault)?.id
      }
      if (!targetActionId) return

      try {
        await bridgeConfirm({
          kind: 'candidate',
          candidateId: 0,
          actionId: targetActionId,
          queryText: query.value,
          generation: currentGeneration.value,
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
        kind: 'candidate',
        candidateId: item.id,
        actionId: targetActionId,
        queryText: query.value,
        generation: currentGeneration.value,
      })
    } catch (e) {
      console.error('[doConfirm] Search action failed:', e)
      return
    }

    // 后端裁决进入参数面板（参数缺失判定在 Dispatcher）：响应载荷自包含
    // candidateId + userArgCount（核心程序专属形态），前端据此构造输入字段，
    // 无需依赖列表项。
    if (resp.status === 'enterParamPanel') {
      // 后端确认响应携带最新代际（进入参数面板是新的会话投影），单调递增更新
      if (resp.generation >= currentGeneration.value) {
        currentGeneration.value = resp.generation
      }
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
        kind: 'candidate',
        candidateId,
        actionId: 'execute',
        queryText: inlineParamState.value.triggerKeyword,
        userArgs: args,
        generation: currentGeneration.value,
      })
    } catch (e) {
      console.error('[confirmInlineParam] failed:', e)
      return
    }

    resetSessionAndHide()
  }

  // ---- 参数面板模式 ----

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
        kind: 'candidate',
        candidateId,
        actionId: 'execute',
        queryText: query.value,
        userArgs,
        generation: currentGeneration.value,
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

  /// Enter 宿主解释（confirm）：按面板状态分流——
  /// 面板已有可执行动作（如翻译成功）→ 执行默认动作（复制译文）；
  /// 否则（ready/失败/空）→ 发起确认查询（翻译或失败后重试）。
  /// 约定：插件仅在结果可执行时返回非空动作列表（translator 成功含 copy_primary，失败/ready 为空）。
  /// 确认查询在途：忽略重复 Enter（不发查询=不加序号），首次结果返回后自然显示。
  /// 「已开始翻译」提示由面板自监听 panelQueryInFlight，此处不再置任何 store 标志。
  function confirmQuery() {
    // 在途防重：确认查询在途，或面板自动查询（live 翻译）在途时忽略 Enter——
    // 避免 onInput 模式空面板/翻译在途按 Enter 触发重复 LLM 调用（回归修复）。
    if (confirmInFlight.value || panelQueryInFlight.value) return
    if (panelActions.value.length > 0) {
      doConfirm()
      return
    }
    confirmInFlight.value = true
    void doQuery(query.value, true)
  }

  /// 退出插件面板（宿主默认 Escape）：清空面板状态并回到搜索。
  /// 统一行内/全页面插件退出语义；后端模式由下一次 bridge_query 自然重置。
  function back() {
    panelType.value = null
    panelData.value = null
    panelActions.value = []
    currentPluginId.value = null
    doQuery('')
  }

  // ---- 会话管理 ----

  function hideWindow() {
    // 隐藏窗口视为放弃当前输入：取消挂起防抖，避免窗口隐藏后仍触发查询
    cancelPendingDebounce()
    bridgeHideWindow().catch((e) => console.warn('[hideWindow] Failed to hide window:', e))
  }

  function resetLocalSession() {
    // 取消未触发的防抖定时器
    cancelPendingDebounce()
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
    currentPluginId.value = null
    confirmInFlight.value = false
    inlineParamState.value = null
    paramPanelState.value = null
    selectedIndex.value = 0
    selectedActionIndex.value = 0
    panelQueryInFlight.value = false
  }

  function resetSessionAndHide() {
    resetLocalSession()
    hideWindow()
  }

  function selectNext() {
    const n = results.value.length
    if (n === 0) return
    // 循环导航：到底自动回到开头（对齐老版 main_search_shortcut_handler 的取模语义）
    selectedIndex.value = (selectedIndex.value + 1) % n
  }

  function selectPrev() {
    const n = results.value.length
    if (n === 0) return
    // 循环导航：到顶自动回到末尾
    selectedIndex.value = (selectedIndex.value - 1 + n) % n
  }

  async function refreshCandidates(): Promise<number> {
    cachedCount.value = await bridgeRefreshCandidates()
    return cachedCount.value
  }

  async function fetchCandidatesCount() {
    cachedCount.value = await bridgeGetCandidatesCount()
  }

  /** 全量刷新插件元数据缓存（不可变更新：整体替换为新引用）。
   *  由 useKeyboardRouter 在插件列表刷新时调用，数据源为后端 plugin_list。 */
  function updatePluginMeta(
    list: Array<{ pluginId: string; name: string; icon: string | null; mode: 'inline' | 'panel' }>,
  ) {
    const next: Record<string, { name: string; icon: string | null; mode: 'inline' | 'panel' }> = {}
    for (const p of list) {
      next[p.pluginId] = { name: p.name, icon: p.icon, mode: p.mode }
    }
    pluginMeta.value = next
  }

  /// 应用后端会话状态事件（会话系统唯一事件通道）。
  /// 无条件接受：事件总是描述当前会话最新投影，按事件内容覆盖交互契约/触发词/插件 ID；
  /// 会话结束（presentation 'none'）→ 复位本地会话，但保留 currentGeneration 单调性。
  /// generation 单调递增更新：旧代际事件不倒退。
  function applySessionState(event: SessionStateEvent) {
    if (event.generation >= currentGeneration.value) {
      currentGeneration.value = event.generation
    }
    panelInteraction.value = event.interaction ?? null
    panelTriggerKeywords = event.triggerKeywords
    currentPluginId.value = event.panel?.pluginId ?? null
    if (event.presentation === 'none') {
      resetLocalSession()
      return
    }
    // 热键唤醒推送携带面板渲染载荷（窗口隐藏时无查询响应可依赖）：
    // 直接按事件重建插件面板会话；常规路径（关键词查询）panelContent 为 null，
    // 载荷仍由 bridge_query 响应下发，此处不覆盖既有面板状态。
    if (event.panelContent) {
      sessionMode.value =
        event.presentation === 'pluginImmersive' ? 'plugin_immersive' : 'plugin_panel'
      panelType.value = event.panelContent.panelType
      panelData.value = event.panelContent.data
      panelActions.value = event.panelContent.actions
      query.value = ''
      results.value = []
      selectedIndex.value = 0
      selectedActionIndex.value = 0
      panelQueryInFlight.value = false
      inlineParamState.value = null
      paramPanelState.value = null
    }
  }

  // 监听后端会话状态事件：会话投影每次变化推送（含插件面板路由命中、会话结束）。
  onSessionState((event) => {
    applySessionState(event)
  })

  return {
    query, results, selectedIndex, selectedActionIndex, sessionMode, cachedCount,
    panelType, panelData, panelActions, panelInteraction,
    currentGeneration, currentPluginId, pluginMeta,
    panelQueryInFlight,
    confirmInFlight,
    inlineParamState, paramPanelState,
    isIdle, selectedItem,
    doQuery, doConfirm, selectNext, selectPrev,
    refreshCandidates, fetchCandidatesCount, hideWindow, updatePluginMeta,
    // 行内参数模式
    exitInlineParamMode, confirmInlineParam,
    // 参数面板模式
    exitParamPanelMode, confirmParamPanel, paramPanelFocusNext, paramPanelFocusPrev,
    // 宿主面板按键动作
    confirmQuery, back,
    // 会话
    applySessionState,
  }
})
