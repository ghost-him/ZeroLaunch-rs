import { Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { AppConfig, ShortcutConfig, Shortcut } from '../api/remote_config_types'

// Define interfaces for the components we interact with
export interface EverythingPanelInstance {
  moveSelection: (direction: number) => void
  launchSelected: () => void
}

export interface SearchBarInstance {
  realInputRef: HTMLInputElement | null
  focus: () => void
}

export interface ResultListInstance {
  resultsListRef: HTMLElement | null
}

export interface SubMenuInstance {
  isVisible: () => boolean
  hideMenu: () => void
  showMenu: (pos: { top: number, left: number }) => void
  selectNext: () => void
  selectPrevious: () => void
  selectCurrent: () => void
}

enum ActionType {
  MOVE_DOWN,
  MOVE_UP,
  MOVE_RIGHT,
  MOVE_LEFT,
  CONFIRM,
  ESCAPE
}

export function useShortcuts(
  app_config: Ref<AppConfig>,
  shortcut_config: Ref<ShortcutConfig>,
  ui_config: Ref<any>,
  isEverythingMode: Ref<boolean>,
  toggleEverythingMode: () => void,
  everythingPanelRef: Ref<EverythingPanelInstance | null>,
  resultsListRef: Ref<ResultListInstance | null>,
  resultItemMenuRef: Ref<SubMenuInstance | null>,
  searchBarRef: Ref<SearchBarInstance | null>,
  searchText: Ref<string>,
  selectedIndex: Ref<number>,
  is_alt_pressed: Ref<boolean>,
  latest_launch_program: Ref<Array<[number, string]>>,
  searchResults: Ref<Array<[number, string]>>,
  launch_program: (index: number, ctrlKey?: boolean, shiftKey?: boolean) => Promise<void>,
  confirmParameterInput: () => Promise<void>,
  cancelParameterSession: () => void,
  parameterSession: Ref<any>,
  handleRightArrowCallback: (event: KeyboardEvent) => void
) {

  const isScrollMode = () => {
    const currentResults = is_alt_pressed.value ? latest_launch_program.value : searchResults.value
    return currentResults.length > app_config.value.scroll_threshold
  }

  const scrollToSelectedItem = () => {
    if (!resultsListRef.value?.resultsListRef || !isScrollMode()) return
    
    const container = resultsListRef.value.resultsListRef
    const itemHeight = ui_config.value.result_item_height
    const selectedItemTop = selectedIndex.value * itemHeight
    const selectedItemBottom = selectedItemTop + itemHeight
    const containerScrollTop = container.scrollTop
    const containerHeight = container.clientHeight
    const containerScrollBottom = containerScrollTop + containerHeight
  
    let targetScrollTop = null
  
    if (selectedItemTop < containerScrollTop) {
      targetScrollTop = selectedItemTop
    }
    else if (selectedItemBottom > containerScrollBottom) {
      targetScrollTop = selectedItemBottom - containerHeight
    }
  
    if (targetScrollTop !== null) {
      container.scrollTo({
        top: targetScrollTop,
        behavior: 'smooth',
      })
    }
  }

  const handleAction = (
    action: ActionType,
    isMenuVisible: boolean,
    ctrlKey: boolean = false,
    shiftKey: boolean = false,
  ) => {
    switch (action) {
      case ActionType.MOVE_DOWN:
        if (isMenuVisible) {
          resultItemMenuRef.value?.selectNext()
        } else {
          const currentResults = is_alt_pressed.value ? latest_launch_program.value : searchResults.value
          const count = Math.min(currentResults.length, app_config.value.search_result_count)
          if (count > 0) {
             selectedIndex.value = (selectedIndex.value + 1) % count
             scrollToSelectedItem()
          }
        }
        break
  
      case ActionType.MOVE_UP:
        if (isMenuVisible) {
          resultItemMenuRef.value?.selectPrevious()
        } else {
          const currentResults = is_alt_pressed.value ? latest_launch_program.value : searchResults.value
          const maxIndex = Math.min(currentResults.length, app_config.value.search_result_count)
          if (maxIndex > 0) {
            selectedIndex.value = (selectedIndex.value - 1 + maxIndex) % maxIndex
            scrollToSelectedItem()
          }
        }
        break
  
      case ActionType.MOVE_RIGHT:
        if (!isMenuVisible) {
          handleRightArrowCallback(new KeyboardEvent('keydown'))
        }
        break
  
      case ActionType.MOVE_LEFT:
        if (isMenuVisible) {
          resultItemMenuRef.value?.hideMenu()
        }
        break
  
      case ActionType.CONFIRM:
        if (isMenuVisible) {
          resultItemMenuRef.value?.selectCurrent()
        } else {
          launch_program(selectedIndex.value, ctrlKey, shiftKey)
        }
        break
  
      case ActionType.ESCAPE:
        if ((searchText.value.length === 0 && !isMenuVisible) ||
          app_config.value.is_esc_hide_window_priority) {
          invoke('hide_window').catch(console.error)
        } else {
          if (isMenuVisible) {
            resultItemMenuRef.value?.hideMenu()
          } else {
            searchText.value = ''
          }
        }
        break
    }
  }

  const matchShortcut = (event: KeyboardEvent, shortcutConfig: Shortcut): boolean => {
    return event.key.toLowerCase() === shortcutConfig.key.toLowerCase() &&
      event.ctrlKey === shortcutConfig.ctrl &&
      event.shiftKey === shortcutConfig.shift &&
      event.metaKey === shortcutConfig.meta
  }

  const preventDefaultWebViewShortcuts = (event: KeyboardEvent) => {
    if (event.key === 'F5' || (event.ctrlKey && event.key.toLowerCase() === 'r')) {
      event.preventDefault()
    }
    if (event.ctrlKey && event.key.toLowerCase() === 'p') {
      event.preventDefault()
    }
    if (event.ctrlKey && ['=', '-', '0'].includes(event.key)) {
      event.preventDefault()
    }
    if (event.ctrlKey && ['f', 's'].includes(event.key.toLowerCase())) {
      event.preventDefault()
    }
  }

  const handleKeyDown = async (event: KeyboardEvent) => {
    preventDefaultWebViewShortcuts(event)
    const isMenuVisible = resultItemMenuRef.value?.isVisible() || false
    
    if (parameterSession.value) {
      if (event.key === 'Escape') {
        event.preventDefault()
        cancelParameterSession()
        return
      }
      if (event.key === 'Enter') {
        event.preventDefault()
        await confirmParameterInput()
        return
      }
      return
    }
  
    // Switch to Everything mode shortcut
    if (matchShortcut(event, shortcut_config.value.switch_to_everything)) {
      event.preventDefault()
      toggleEverythingMode()
      return
    }
  
    if (isEverythingMode.value) {
      // In Everything mode, we also want to support configured shortcuts for navigation
      if (event.key === 'ArrowDown' || matchShortcut(event, shortcut_config.value.arrow_down)) {
        event.preventDefault()
        everythingPanelRef.value?.moveSelection(1)
        return
      }
      if (event.key === 'ArrowUp' || matchShortcut(event, shortcut_config.value.arrow_up)) {
        event.preventDefault()
        everythingPanelRef.value?.moveSelection(-1)
        return
      }
      if (event.key === 'Enter') {
        event.preventDefault()
        everythingPanelRef.value?.launchSelected()
        return
      }
      // Allow other keys (like typing) to pass through
      if (event.key !== 'Alt' && event.key !== 'Control' && event.key !== 'Shift') {
          // Let it bubble to input
      }
      return
    }
  
    if (event.key === 'Alt') {
      is_alt_pressed.value = true
      event.preventDefault()
    }
  
    if (event.key === 'ArrowDown' || matchShortcut(event, shortcut_config.value.arrow_down)) {
      event.preventDefault()
      handleAction(ActionType.MOVE_DOWN, isMenuVisible)
      return
    }
  
    if (event.key === 'ArrowUp' || matchShortcut(event, shortcut_config.value.arrow_up)) {
      event.preventDefault()
      handleAction(ActionType.MOVE_UP, isMenuVisible)
      return
    }
  
    if (event.key === 'ArrowRight' || matchShortcut(event, shortcut_config.value.arrow_right)) {
      const inputElement = searchBarRef.value?.realInputRef
      const isAtEnd = inputElement && (inputElement.selectionStart === searchText.value.length)
  
      if (!isMenuVisible && isAtEnd && document.activeElement === inputElement) {
        event.preventDefault()
        handleAction(ActionType.MOVE_RIGHT, isMenuVisible)
      }
      return
    }
  
    if (event.key === 'ArrowLeft' || matchShortcut(event, shortcut_config.value.arrow_left)) {
      if (isMenuVisible) {
        event.preventDefault()
        handleAction(ActionType.MOVE_LEFT, isMenuVisible)
      }
      return
    }
  
    if (event.key === 'Enter' || (event.key === ' ' && app_config.value.space_is_enter)) {
      event.preventDefault()
      handleAction(ActionType.CONFIRM, isMenuVisible, event.ctrlKey, event.shiftKey)
      return
    }
  
    if (event.key === 'Escape') {
      handleAction(ActionType.ESCAPE, isMenuVisible)
      return
    }
  }

  const handleKeyUp = (event: KeyboardEvent) => {
    if (event.key === 'Alt') {
      is_alt_pressed.value = false
    }
  }

  const handleBlur = () => {
    is_alt_pressed.value = false
  }

  return {
    handleKeyDown,
    handleKeyUp,
    handleBlur
  }
}
