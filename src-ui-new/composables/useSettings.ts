import { WebviewWindow } from '@tauri-apps/api/webviewWindow'

export function useSettings() {
  async function openSettings() {
    const win = await WebviewWindow.getByLabel('setting_window')
    if (win) {
      await win.show()
      await win.setFocus()
    }
  }

  return { openSettings }
}
