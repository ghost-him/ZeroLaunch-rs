import type { ShortcutHandler } from './shortcut_handler'

/**
 * 参数输入面板实例接口
 */
export interface ParameterPanelInstance {
    focus: () => void
}

/**
 * 创建参数输入模式的快捷键处理器
 * @param confirmParameterInput 确认参数输入的回调
 * @param cancelParameterSession 取消参数会话的回调
 * @returns 快捷键处理器实例
 */
export function createParameterInputShortcutHandler(
    confirmParameterInput: () => Promise<void>,
    cancelParameterSession: () => void,
): ShortcutHandler {
    return {
        handleKeyDown(event: KeyboardEvent): boolean {
            // ESC 取消参数输入
            if (event.key === 'Escape') {
                event.preventDefault()
                cancelParameterSession()
                return true
            }

            // Enter 确认参数输入
            if (event.key === 'Enter') {
                event.preventDefault()
                confirmParameterInput()
                return true
            }

            // 其他键不处理，让输入框接收
            return false
        },
    }
}
