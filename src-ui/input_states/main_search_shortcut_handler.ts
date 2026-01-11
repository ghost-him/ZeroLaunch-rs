import type { Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { AppConfig, ShortcutConfig, UIConfig } from '../api/remote_config_types'
import type { ShortcutHandler } from './shortcut_handler'
import { matchShortcut } from './shortcut_handler'

/**
 * 搜索栏实例接口
 */
export interface SearchBarInstance {
    realInputRef: HTMLInputElement | null
    focus: () => void
}

/**
 * 结果列表实例接口
 */
export interface ResultListInstance {
    resultsListRef: HTMLElement | null
}

/**
 * 子菜单实例接口
 */
export interface SubMenuInstance {
    isVisible: () => boolean
    hideMenu: () => void
    showMenu: (pos: { top: number; left: number }) => void
    selectNext: () => void
    selectPrevious: () => void
    selectCurrent: () => void
}

/**
 * 主搜索页面快捷键处理器的配置选项
 */
export interface MainSearchShortcutHandlerOptions {
    appConfig: Ref<AppConfig>
    shortcutConfig: Ref<ShortcutConfig>
    uiConfig: Ref<UIConfig>
    resultsListRef: Ref<ResultListInstance | null>
    resultItemMenuRef: Ref<SubMenuInstance | null>
    searchBarRef: Ref<SearchBarInstance | null>
    searchText: Ref<string>
    selectedIndex: Ref<number>
    isAltPressed: Ref<boolean>
    latestLaunchProgram: Ref<Array<[number, string, string]>>
    searchResults: Ref<Array<[number, string, string]>>
    launchProgram: (index: number, ctrlKey?: boolean, shiftKey?: boolean) => Promise<void>
    handleRightArrowCallback: (event: KeyboardEvent) => void
}

/**
 * 创建主搜索页面的快捷键处理器
 * @param options 配置选项
 * @returns 快捷键处理器实例
 */
export function createMainSearchShortcutHandler(
    options: MainSearchShortcutHandlerOptions,
): ShortcutHandler {
    const {
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
    } = options

    /**
     * 判断当前是否处于滚动模式
     */
    const isScrollMode = (): boolean => {
        const currentResults = isAltPressed.value ? latestLaunchProgram.value : searchResults.value
        return currentResults.length > appConfig.value.scroll_threshold
    }

    /**
     * 滚动到选中项
     */
    const scrollToSelectedItem = (): void => {
        if (!resultsListRef.value?.resultsListRef || !isScrollMode()) return

        const container = resultsListRef.value.resultsListRef
        const itemHeight = uiConfig.value.result_item_height
        const selectedItemTop = selectedIndex.value * itemHeight
        const selectedItemBottom = selectedItemTop + itemHeight
        const containerScrollTop = container.scrollTop
        const containerHeight = container.clientHeight
        const containerScrollBottom = containerScrollTop + containerHeight

        let targetScrollTop: number | null = null

        if (selectedItemTop < containerScrollTop) {
            targetScrollTop = selectedItemTop
        } else if (selectedItemBottom > containerScrollBottom) {
            targetScrollTop = selectedItemBottom - containerHeight
        }

        if (targetScrollTop !== null) {
            container.scrollTo({
                top: targetScrollTop,
                behavior: 'smooth',
            })
        }
    }

    /**
     * 获取当前结果的数量（受限于最大显示数）
     */
    const getCurrentResultCount = (): number => {
        const currentResults = isAltPressed.value ? latestLaunchProgram.value : searchResults.value
        return Math.min(currentResults.length, appConfig.value.search_result_count)
    }

    /**
     * 处理向下移动
     */
    const handleMoveDown = (isMenuVisible: boolean): void => {
        if (isMenuVisible) {
            resultItemMenuRef.value?.selectNext()
        } else {
            const count = getCurrentResultCount()
            if (count > 0) {
                selectedIndex.value = (selectedIndex.value + 1) % count
                scrollToSelectedItem()
            }
        }
    }

    /**
     * 处理向上移动
     */
    const handleMoveUp = (isMenuVisible: boolean): void => {
        if (isMenuVisible) {
            resultItemMenuRef.value?.selectPrevious()
        } else {
            const maxIndex = getCurrentResultCount()
            if (maxIndex > 0) {
                selectedIndex.value = (selectedIndex.value - 1 + maxIndex) % maxIndex
                scrollToSelectedItem()
            }
        }
    }

    /**
     * 处理向右移动（显示子菜单）
     */
    const handleMoveRight = (isMenuVisible: boolean): void => {
        if (!isMenuVisible) {
            handleRightArrowCallback(new KeyboardEvent('keydown'))
        }
    }

    /**
     * 处理向左移动（关闭子菜单）
     */
    const handleMoveLeft = (isMenuVisible: boolean): void => {
        if (isMenuVisible) {
            resultItemMenuRef.value?.hideMenu()
        }
    }

    /**
     * 处理确认操作
     */
    const handleConfirm = (isMenuVisible: boolean, ctrlKey: boolean, shiftKey: boolean): void => {
        if (isMenuVisible) {
            resultItemMenuRef.value?.selectCurrent()
        } else {
            launchProgram(selectedIndex.value, ctrlKey, shiftKey)
        }
    }

    /**
     * 处理 ESC 键
     */
    const handleEscape = (isMenuVisible: boolean): void => {
        if (
            (searchText.value.length === 0 && !isMenuVisible) ||
            appConfig.value.is_esc_hide_window_priority
        ) {
            invoke('hide_window').catch(console.error)
        } else {
            if (isMenuVisible) {
                resultItemMenuRef.value?.hideMenu()
            } else {
                searchText.value = ''
            }
        }
    }

    return {
        handleKeyDown(event: KeyboardEvent): boolean {
            const isMenuVisible = resultItemMenuRef.value?.isVisible() || false

            // Alt 键：切换到最近使用程序列表
            if (event.key === 'Alt') {
                isAltPressed.value = true
                event.preventDefault()
                return true
            }

            // 向下移动
            if (event.key === 'ArrowDown' || matchShortcut(event, shortcutConfig.value.arrow_down)) {
                event.preventDefault()
                handleMoveDown(isMenuVisible)
                return true
            }

            // 向上移动
            if (event.key === 'ArrowUp' || matchShortcut(event, shortcutConfig.value.arrow_up)) {
                event.preventDefault()
                handleMoveUp(isMenuVisible)
                return true
            }

            // 向右移动（显示子菜单）
            if (event.key === 'ArrowRight' || matchShortcut(event, shortcutConfig.value.arrow_right)) {
                const inputElement = searchBarRef.value?.realInputRef
                const isAtEnd = inputElement && inputElement.selectionStart === searchText.value.length

                if (!isMenuVisible && isAtEnd && document.activeElement === inputElement) {
                    event.preventDefault()
                    handleMoveRight(isMenuVisible)
                    return true
                }
                return false
            }

            // 向左移动（关闭子菜单）
            if (event.key === 'ArrowLeft' || matchShortcut(event, shortcutConfig.value.arrow_left)) {
                if (isMenuVisible) {
                    event.preventDefault()
                    handleMoveLeft(isMenuVisible)
                    return true
                }
                return false
            }

            // 确认选择
            if (event.key === 'Enter' || (event.key === ' ' && appConfig.value.space_is_enter)) {
                event.preventDefault()
                handleConfirm(isMenuVisible, event.ctrlKey, event.shiftKey)
                return true
            }

            // ESC 键
            if (event.key === 'Escape') {
                handleEscape(isMenuVisible)
                return true
            }

            // 自动聚焦和输入重定向：如果输入框未聚焦，尝试捕获输入
            const inputElement = searchBarRef.value?.realInputRef
            if (inputElement && document.activeElement !== inputElement) {
                // 处理普通字符输入
                if (event.key.length === 1 && !event.ctrlKey && !event.altKey && !event.metaKey) {
                    event.preventDefault()
                    searchBarRef.value?.focus()
                    searchText.value += event.key
                    return true
                }
                // 处理退格键
                if (event.key === 'Backspace') {
                    event.preventDefault()
                    searchBarRef.value?.focus()
                    searchText.value = searchText.value.slice(0, -1)
                    return true
                }
            }

            // 未处理的事件
            return false
        },

        handleKeyUp(event: KeyboardEvent): void {
            if (event.key === 'Alt') {
                isAltPressed.value = false
            }
        },
    }
}
