import { defineStore } from 'pinia'
import { ref } from 'vue'
import { darkTheme, type GlobalTheme } from 'naive-ui'
import { configGetSettings, configApplySettings } from '@/bridge/commands'
import { applyAppearanceSettings, extractPlaceholder } from '@/utils/appearance'

export type ThemeMode = 'system' | 'light' | 'dark'
export type Locale = 'zh-Hans' | 'en'

function detectSystemPreference(): boolean {
  try {
    return window.matchMedia('(prefers-color-scheme: dark)').matches
  } catch {
    return false
  }
}

export const useThemeStore = defineStore('theme', () => {
  const themeMode = ref<ThemeMode>('system')
  const systemIsDark = ref(detectSystemPreference())
  const naiveTheme = ref<GlobalTheme | null>(null)
  const locale = ref<Locale>('zh-Hans')

  /** 搜索栏占位符文本（响应式，直接绑定到 SearchBar 的 placeholder 属性） */
  const searchBarPlaceholder = ref('Hello, ZeroLaunch! ヾ(≧▽≦*)o')

  let systemMediaQuery: MediaQueryList | null = null

  function onSystemChange(e: MediaQueryListEvent) {
    systemIsDark.value = e.matches
    if (themeMode.value === 'system') {
      applyNaiveTheme()
    }
  }

  function applyNaiveTheme() {
    const dark = themeMode.value === 'dark' || (themeMode.value === 'system' && systemIsDark.value)
    naiveTheme.value = dark ? darkTheme : null
    document.documentElement.classList.toggle('dark', dark)

    // 主题切换时重新应用配色 CSS 变量
    if (Object.keys(currentAppearanceSettings).length > 0) {
      applyAppearanceSettings(currentAppearanceSettings)
    }
  }

  /** 当前内存中的外观配置缓存（用于主题切换时重新应用配色） */
  let currentAppearanceSettings: Record<string, unknown> = {}

  function setTheme(mode: ThemeMode) {
    themeMode.value = mode
    applyNaiveTheme()
    syncToBackend(mode)
  }

  let syncTimer: ReturnType<typeof setTimeout> | null = null
  function syncToBackend(mode: ThemeMode) {
    if (syncTimer) clearTimeout(syncTimer)
    syncTimer = setTimeout(async () => {
      try {
        const current = await configGetSettings('appearance').catch(() => ({}))
        const merged = { ...(current as Record<string, unknown>), theme: mode }
        await configApplySettings('appearance', merged)
      } catch (e) {
        console.warn('[theme-store] Failed to sync theme to backend:', e)
      }
    }, 100)
  }

  /** 从后端加载外观配置（主题 + 语言 + 全部外观设置），在应用挂载前调用 */
  async function loadFromBackend(): Promise<Locale> {
    let lang: Locale = 'zh-Hans'
    try {
      const settings = await configGetSettings('appearance')
      const s = settings as Record<string, unknown> | undefined
      const t = (s?.theme as ThemeMode | undefined) ?? 'system'
      themeMode.value = t
      if (s?.language === 'en') {
        lang = 'en'
      }
      locale.value = lang

      // 应用外观 CSS 变量并同步响应式状态
      if (s) {
        currentAppearanceSettings = s
        applyAppearanceSettings(s)
        searchBarPlaceholder.value = extractPlaceholder(s)
      }
    } catch {
      console.warn('[theme-store] Failed to load appearance config, using defaults')
      themeMode.value = 'system'
    }
    applyNaiveTheme()

    systemMediaQuery = window.matchMedia('(prefers-color-scheme: dark)')
    systemMediaQuery.addEventListener('change', onSystemChange)

    return lang
  }

  /** 应用跨窗口同步的外观配置（不写回后端，避免死循环） */
  function applyRemoteSettings(settings: Record<string, unknown>) {
    const t = (settings.theme as ThemeMode | undefined) ?? themeMode.value
    const themeChanged = t !== themeMode.value
    if (themeChanged) {
      themeMode.value = t
      applyNaiveTheme()
    }
    const l = settings.language === 'en' ? 'en' : 'zh-Hans'
    const langChanged = l !== locale.value
    if (langChanged) {
      locale.value = l
    }

    // 重新应用外观 CSS 变量并同步响应式状态
    currentAppearanceSettings = settings
    applyAppearanceSettings(settings)
    searchBarPlaceholder.value = extractPlaceholder(settings)

    return { themeChanged, langChanged, newLang: l }
  }

  function stopSystemListener() {
    systemMediaQuery?.removeEventListener('change', onSystemChange)
  }

  return {
    themeMode,
    naiveTheme,
    locale,
    searchBarPlaceholder,
    setTheme,
    loadFromBackend,
    applyRemoteSettings,
    stopSystemListener,
  }
})
