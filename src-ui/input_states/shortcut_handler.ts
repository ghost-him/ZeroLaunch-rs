import type { Shortcut } from '../api/remote_config_types'

/**
 * 快捷键处理器接口
 * 每个输入上下文可以有自己的快捷键处理器实现
 */
export interface ShortcutHandler {
    /**
     * 尝试处理按键事件
     * @param event 键盘事件
     * @returns 返回 true 表示事件已被处理，false 表示未处理
     */
    handleKeyDown(event: KeyboardEvent): boolean

    /**
     * 处理按键释放（可选）
     * @param event 键盘事件
     */
    handleKeyUp?(event: KeyboardEvent): void
}

/**
 * 匹配快捷键配置与键盘事件
 * @param event 键盘事件
 * @param shortcut 快捷键配置
 * @returns 是否匹配
 */
export function matchShortcut(event: KeyboardEvent, shortcut: Shortcut): boolean {
    return (
        event.key.toLowerCase() === shortcut.key.toLowerCase() &&
        event.ctrlKey === shortcut.ctrl &&
        event.shiftKey === shortcut.shift &&
        event.altKey === shortcut.alt &&
        event.metaKey === shortcut.meta
    )
}
