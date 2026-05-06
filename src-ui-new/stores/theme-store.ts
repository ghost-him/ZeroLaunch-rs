import { defineStore } from 'pinia'
import { ref } from 'vue'
import { darkTheme, type GlobalTheme } from 'naive-ui'
import { configGetSettings, configApplySettings } from '@/bridge/commands'

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
  }

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

  /** 从后端加载外观配置（主题 + 语言），在应用挂载前调用 */
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
    if (t !== themeMode.value) {
      themeMode.value = t
      applyNaiveTheme()
    }
    const l = settings.language === 'en' ? 'en' : 'zh-Hans'
    if (l !== locale.value) {
      locale.value = l
    }
    return { themeChanged: t !== themeMode.value, langChanged: l !== locale.value, newLang: l }
  }

  function stopSystemListener() {
    systemMediaQuery?.removeEventListener('change', onSystemChange)
  }

  return {
    themeMode,
    naiveTheme,
    locale,
    setTheme,
    loadFromBackend,
    applyRemoteSettings,
    stopSystemListener,
  }
})
