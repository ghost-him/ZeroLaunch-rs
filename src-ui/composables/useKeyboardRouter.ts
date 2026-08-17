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
      // 插件元数据缓存：供 Footer / 搜索栏前缀渲染当前插件标识
      store.updatePluginMeta(list)
      // 热键表：仅 panel 形态插件参与热键唤醒（行内插件不注册）
      // 同热键多插件冲突：仅保留排序靠前的一个（后端按优先级/ID 排序，确定性），
      // 其余丢弃并告警——冲突热键不可达是确定行为，避免静默竞争。
      const seen = new Set<string>()
      const entries: PluginHotkeyEntry[] = []
      for (const p of list) {
        if (!p.enabled || p.mode !== 'panel' || !p.hotkey) continue
        if (seen.has(p.hotkey)) {
          console.warn(
            `[keyboard] 插件 ${p.pluginId} 的热键 ${p.hotkey} 与已启用插件冲突，已忽略（仅排序靠前的插件生效）`,
          )
          continue
        }
        seen.add(p.hotkey)
        entries.push({ pluginId: p.pluginId, hotkey: p.hotkey })
      }
      pluginHotkeys.value = entries
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

  /// 插件热键唤醒在途标记（plugin_id → 唤醒 IPC 尚未返回）。
  /// 按住组合键触发系统自动重复 keydown 时，防止并发重复唤醒同一插件
  /// （重复 IPC 既浪费，也会放大慢响应乱序覆盖会话的问题）。
  const wakeInFlight = new Set<string>()

  function onKeyDown(e: KeyboardEvent) {
    // 插件热键（窗口级全局，优先于会话面板分发）：命中已启用插件的声明热键 →
    // 唤醒该插件（沉浸式全页面接管）。已在同一插件面板时不重复唤醒，放行给面板绑定。
    for (const p of pluginHotkeys.value) {
      if (!matchesKey(e, p.hotkey)) continue
      const inSamePanel =
        store.currentPluginId === p.pluginId &&
        (store.sessionMode === 'plugin_panel' || store.sessionMode === 'plugin_immersive')
      if (inSamePanel) break
      // 按住自动重复或上次唤醒仍在途：吞掉事件不再发起新唤醒
      if (e.repeat || wakeInFlight.has(p.pluginId)) {
        e.preventDefault()
        return
      }
      e.preventDefault()
      wakeInFlight.add(p.pluginId)
      void bridgeWakePlugin(p.pluginId)
        .catch((e) => {
          console.warn('[keyboard] 插件热键唤醒失败:', e)
          // 自愈：卸载/禁用等场景热键表残留过期条目，失败后刷新
          void refreshPluginHotkeys()
        })
        .finally(() => {
          wakeInFlight.delete(p.pluginId)
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
