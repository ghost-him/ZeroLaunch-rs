import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useSearchStore, type SessionMode } from '@/stores/search-store'
import { useConfigStore } from '@/stores/config-store'
import { dispatchKeyDown, matchesKey } from './keyboard/registry'
import type { KeyOpts } from './keyboard/types'
import { bridgeWakePlugin, pluginList } from '@/bridge/commands'
import { onPluginInstalled, onPluginUninstalled, onShowWindow } from '@/bridge/events'

/// 插件热键映射：plugin_id → 声明热键（仅已启用插件；数据源为后端 plugin_list）。
/// 插件唤醒快捷键只在前端处理（窗口唤起后按键才可达），不注册 OS 全局热键。
interface PluginHotkeyEntry {
  pluginId: string
  hotkey: string
}

export function useKeyboardRouter() {
  const store = useSearchStore()
  const configStore = useConfigStore()

  const uiMode = computed<SessionMode>(() => store.sessionMode)

  /// 当前生效的插件热键表。窗口每次唤起与插件安装/卸载后刷新：
  /// 热键仅需在窗口内生效，无需 OS 注册，故不用实时推送，下次唤起自然最新。
  const pluginHotkeys = ref<PluginHotkeyEntry[]>([])

  async function refreshPluginHotkeys() {
    try {
      const list = await pluginList()
      pluginHotkeys.value = list
        .filter((p) => p.enabled && p.hotkey)
        .map((p) => ({ pluginId: p.pluginId, hotkey: p.hotkey as string }))
    } catch (e) {
      console.warn('[keyboard] 刷新插件热键失败:', e)
    }
  }

  /// Windows 菜单模式拦截：单独按下 Alt 会激活窗口菜单模式（WM_SYSKEYDOWN），
  /// 之后第一个按键的 keydown 会被系统当作菜单助记符吞掉（只有 keyup 到达），
  /// 表现为"按过 Alt 后第一个字母无法输入"。
  /// 在捕获阶段对 Alt 键事件 preventDefault，阻止菜单模式激活。
  /// 系统级 Alt+Space 热键由 RegisterHotKey 在 DOM 分发之前处理，不受影响；
  /// Alt+字母 组合（插件声明式按键绑定）的字母 keydown 仍正常分发，不受影响。
  /// Ctrl+Alt（AltGr）组合不拦截，避免破坏 AltGr 字符输入。
  function onAltKeyCapture(e: KeyboardEvent) {
    if (e.ctrlKey) return
    if (e.key === 'Alt' || e.code === 'AltLeft' || e.code === 'AltRight') {
      e.preventDefault()
    }
  }

  function onKeyDown(e: KeyboardEvent) {
    // 插件热键（窗口级全局，优先于会话面板分发）：命中已启用插件的声明热键 →
    // 唤醒该插件（沉浸式全页面接管）。已在同一插件面板时不重复唤醒，放行给面板绑定。
    for (const p of pluginHotkeys.value) {
      if (!matchesKey(e, p.hotkey)) continue
      const inSamePanel =
        store.currentPluginId === p.pluginId &&
        (store.sessionMode === 'plugin_panel' || store.sessionMode === 'plugin_immersive')
      if (inSamePanel) break
      e.preventDefault()
      void bridgeWakePlugin(p.pluginId).catch((e) => {
        console.warn('[keyboard] 插件热键唤醒失败:', e)
        // 自愈：卸载/禁用等场景热键表残留过期条目，失败后刷新
        void refreshPluginHotkeys()
      })
      return
    }

    // 键盘解释选项来自 window-behavior-config 设置（缺省 false）
    const wb = configStore.settings['window-behavior-config'] as
      | {
          space_is_enter?: boolean
          is_esc_hide_window_priority?: boolean
          move_up_key?: string
          move_down_key?: string
        }
      | undefined
    const opts: KeyOpts = {
      spaceIsEnter: wb?.space_is_enter ?? false,
      escHideWindowPriority: wb?.is_esc_hide_window_priority ?? false,
      // 上下选择键缺省 Ctrl+K / Ctrl+J（与配置默认值一致）；显式清空后仅保留方向键
      moveUpKey: wb?.move_up_key ?? 'Ctrl+K',
      moveDownKey: wb?.move_down_key ?? 'Ctrl+J',
    }
    dispatchKeyDown(e, store, opts)
  }

  onMounted(() => {
    document.addEventListener('keydown', onAltKeyCapture, true)
    document.addEventListener('keyup', onAltKeyCapture, true)
    document.addEventListener('keydown', onKeyDown)
    // 插件热键表：初始 + 窗口每次唤起 + 插件安装/卸载后刷新
    void refreshPluginHotkeys()
    onShowWindow(() => {
      void refreshPluginHotkeys()
    })
    onPluginInstalled(() => {
      void refreshPluginHotkeys()
    })
    onPluginUninstalled(() => {
      void refreshPluginHotkeys()
    })
  })
  onUnmounted(() => {
    document.removeEventListener('keydown', onAltKeyCapture, true)
    document.removeEventListener('keyup', onAltKeyCapture, true)
    document.removeEventListener('keydown', onKeyDown)
  })

  return { uiMode }
}
