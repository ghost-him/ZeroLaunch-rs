import { WebviewWindow } from '@tauri-apps/api/webviewWindow'
import { cursorPosition, monitorFromPoint } from '@tauri-apps/api/window'
import { PhysicalPosition } from '@tauri-apps/api/dpi'
import { bridgeHideWindow } from '@/bridge/commands'

export function useSettings() {
  async function openSettings() {
    // 先隐藏搜索主窗口，否则 alwaysOnTop 会使设置窗口被遮挡
    await bridgeHideWindow()

    const win = await WebviewWindow.getByLabel('setting_window')
    if (!win) return

    // 以鼠标位置为锚点，找到所在显示器并居中设置窗口
    const cursor = await cursorPosition()
    const monitor = await monitorFromPoint(cursor.x, cursor.y)
    if (monitor) {
      const size = await win.outerSize()
      const x = monitor.position.x + (monitor.size.width - size.width) / 2
      const y = monitor.position.y + (monitor.size.height - size.height) / 2
      await win.setPosition(new PhysicalPosition(Math.round(x), Math.round(y)))
    }

    await win.unminimize()
    await win.show()
    // Windows 前台锁限制下 setFocus（SetForegroundWindow）可能失败：
    // 窗口已打开但被其他窗口遮挡时 show() 不会提升 Z 序。置顶闪切
    // （setAlwaysOnTop true→false）通过 SetWindowPos(HWND_TOPMOST) 强制
    // 提升窗口到前台，再取消置顶使其保持在普通窗口最上层。
    await win.setAlwaysOnTop(true)
    await win.setAlwaysOnTop(false)
    await win.setFocus()
  }

  return { openSettings }
}
