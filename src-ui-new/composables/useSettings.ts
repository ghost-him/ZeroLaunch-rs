import { WebviewWindow } from '@tauri-apps/api/webviewWindow'
import { cursorPosition, monitorFromPoint } from '@tauri-apps/api/window'
import { PhysicalPosition } from '@tauri-apps/api/dpi'

export function useSettings() {
  async function openSettings() {
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

    await win.show()
    await win.setFocus()
  }

  return { openSettings }
}
