import { Ref } from 'vue'
import { AppConfig, ShortcutConfig, EverythingShortcutConfig, UIConfig } from '../api/remote_config_types'
import {
  InputContext,
  matchShortcut,
  createEverythingShortcutHandler,
  createMainSearchShortcutHandler,
  createParameterInputShortcutHandler,
} from '../input_states'
import type {
  EverythingPanelInstance,
  SearchBarInstance,
  ResultListInstance,
  SubMenuInstance,
} from '../input_states'

// Re-export types for backward compatibility
export type { EverythingPanelInstance, SearchBarInstance, ResultListInstance, SubMenuInstance } from '../input_states'

/**
 * useShortcuts 的配置选项
 */
export interface UseShortcutsOptions {
  // 配置
  appConfig: Ref<AppConfig>
  shortcutConfig: Ref<ShortcutConfig>
  everythingShortcutConfig: Ref<EverythingShortcutConfig>
  uiConfig: Ref<UIConfig>

  // 状态
  inputContext: Ref<InputContext>
  searchText: Ref<string>
  selectedIndex: Ref<number>
  isAltPressed: Ref<boolean>
  latestLaunchProgram: Ref<Array<[number, string, string]>>
  searchResults: Ref<Array<[number, string, string]>>

  // 组件引用
  everythingPanelRef: Ref<EverythingPanelInstance | null>
  resultsListRef: Ref<ResultListInstance | null>
  resultItemMenuRef: Ref<SubMenuInstance | null>
  searchBarRef: Ref<SearchBarInstance | null>

  // 回调函数
  toggleEverythingMode: () => void
  launchProgram: (index: number, ctrlKey?: boolean, shiftKey?: boolean) => Promise<void>
  confirmParameterInput: () => Promise<void>
  cancelParameterSession: () => void
  handleRightArrowCallback: (event: KeyboardEvent) => void
}

/**
 * 快捷键管理 Composable
 *
 * 职责：
 * 1. 阻止浏览器默认快捷键
 * 2. 处理全局快捷键（如切换 Everything 模式）
 * 3. 根据当前 InputContext 分发事件到对应的处理器
 */
export function useShortcuts(options: UseShortcutsOptions) {
  const {
    appConfig,
    shortcutConfig,
    everythingShortcutConfig,
    uiConfig,
    inputContext,
    searchText,
    selectedIndex,
    isAltPressed,
    latestLaunchProgram,
    searchResults,
    everythingPanelRef,
    resultsListRef,
    resultItemMenuRef,
    searchBarRef,
    toggleEverythingMode,
    launchProgram,
    confirmParameterInput,
    cancelParameterSession,
    handleRightArrowCallback,
  } = options

  // 创建各个上下文的快捷键处理器
  const mainSearchHandler = createMainSearchShortcutHandler({
    appConfig,
    shortcutConfig,
    uiConfig,
    resultsListRef,
    resultItemMenuRef,
    searchBarRef,
    searchText,
    selectedIndex,
    isAltPressed,
    latestLaunchProgram,
    searchResults,
    launchProgram,
    handleRightArrowCallback,
  })

  const everythingHandler = createEverythingShortcutHandler(
    everythingShortcutConfig,
    shortcutConfig,
    everythingPanelRef,
    searchText,
  )

  const parameterInputHandler = createParameterInputShortcutHandler(
    confirmParameterInput,
    cancelParameterSession,
  )

  /**
   * 阻止 WebView 的默认快捷键行为
   */
  const preventDefaultWebViewShortcuts = (event: KeyboardEvent): void => {
    // 阻止刷新
    if (event.key === 'F5' || (event.ctrlKey && event.key.toLowerCase() === 'r')) {
      event.preventDefault()
    }
    // 阻止打印
    if (event.ctrlKey && event.key.toLowerCase() === 'p') {
      event.preventDefault()
    }
    // 阻止缩放
    if (event.ctrlKey && ['=', '-', '0'].includes(event.key)) {
      event.preventDefault()
    }
    // 阻止查找和保存
    if (event.ctrlKey && ['f', 's', 'i'].includes(event.key.toLowerCase())) {
      event.preventDefault()
    }
  }

  /**
   * 处理全局快捷键（在所有上下文中都生效的快捷键）
   * @returns 如果处理了快捷键则返回 true
   */
  const handleGlobalShortcuts = (event: KeyboardEvent): boolean => {
    // 切换到 Everything 模式（在主搜索和 Everything 模式下都可用）
    if (
      inputContext.value !== InputContext.ParameterInput &&
      matchShortcut(event, shortcutConfig.value.switch_to_everything)
    ) {
      event.preventDefault()
      toggleEverythingMode()
      return true
    }
    return false
  }

  /**
   * 键盘按下事件处理
   */
  const handleKeyDown = async (event: KeyboardEvent): Promise<void> => {
    // 1. 阻止浏览器默认行为
    preventDefaultWebViewShortcuts(event)

    // 2. 处理全局快捷键
    if (handleGlobalShortcuts(event)) {
      return
    }

    // 3. 根据当前上下文分发到对应处理器
    switch (inputContext.value) {
      case InputContext.ParameterInput:
        parameterInputHandler.handleKeyDown(event)
        break

      case InputContext.Everything:
        // Everything 处理器处理
        if (everythingHandler.handleKeyDown(event)) {
          return
        }
        // 未被处理的 ESC：返回主搜索
        if (event.key === 'Escape') {
          event.preventDefault()
          toggleEverythingMode()
        }
        break

      case InputContext.MainSearch:
      default:
        mainSearchHandler.handleKeyDown(event)
        break
    }
  }

  /**
   * 键盘释放事件处理
   */
  const handleKeyUp = (event: KeyboardEvent): void => {
    // 只有主搜索模式需要处理 keyup（Alt 键释放）
    if (inputContext.value === InputContext.MainSearch) {
      mainSearchHandler.handleKeyUp?.(event)
    }
  }

  /**
   * 失去焦点事件处理
   */
  const handleBlur = (): void => {
    // 重置 Alt 键状态
    isAltPressed.value = false
  }

  return {
    handleKeyDown,
    handleKeyUp,
    handleBlur,
  }
}

// ============================================================================
// 兼容性包装器：保持与旧 API 的兼容性
// ============================================================================

/**
 * @deprecated 请使用新的 useShortcuts(options) API
 * 这个函数是为了向后兼容而保留的
 */
export function useShortcutsLegacy(
  app_config: Ref<AppConfig>,
  shortcut_config: Ref<ShortcutConfig>,
  everything_shortcut_config: Ref<EverythingShortcutConfig>,
  ui_config: Ref<any>,
  inputContext: Ref<InputContext>,
  toggleEverythingMode: () => void,
  everythingPanelRef: Ref<EverythingPanelInstance | null>,
  resultsListRef: Ref<ResultListInstance | null>,
  resultItemMenuRef: Ref<SubMenuInstance | null>,
  searchBarRef: Ref<SearchBarInstance | null>,
  searchText: Ref<string>,
  selectedIndex: Ref<number>,
  is_alt_pressed: Ref<boolean>,
  latest_launch_program: Ref<Array<[number, string, string]>>,
  searchResults: Ref<Array<[number, string, string]>>,
  launch_program: (index: number, ctrlKey?: boolean, shiftKey?: boolean) => Promise<void>,
  confirmParameterInput: () => Promise<void>,
  cancelParameterSession: () => void,
  handleRightArrowCallback: (event: KeyboardEvent) => void,
) {
  return useShortcuts({
    appConfig: app_config,
    shortcutConfig: shortcut_config,
    everythingShortcutConfig: everything_shortcut_config,
    uiConfig: ui_config,
    inputContext,
    searchText,
    selectedIndex,
    isAltPressed: is_alt_pressed,
    latestLaunchProgram: latest_launch_program,
    searchResults,
    everythingPanelRef,
    resultsListRef,
    resultItemMenuRef,
    searchBarRef,
    toggleEverythingMode,
    launchProgram: launch_program,
    confirmParameterInput,
    cancelParameterSession,
    handleRightArrowCallback,
  })
}
