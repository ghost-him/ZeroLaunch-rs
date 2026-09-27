import { defineStore } from 'pinia'
import { ref } from 'vue'
import { darkTheme, type GlobalTheme } from 'naive-ui'
import { bridgeGetSystemTheme, configGetSettings } from '@/bridge/commands'
import { onSystemThemeChanged } from '@/bridge/events'
import { applyAppearanceSettings, extractPlaceholder } from '@/utils/appearance'

export type ThemeMode = 'system' | 'light' | 'dark'
export type Locale = 'zh-Hans' | 'zh-Hant' | 'en'

/** 将后端下发的语言值归一化为受支持的语言代码（后端已校验合法值，此处仅兜底） */
function normalizeLocale(lang: unknown): Locale {
  return lang === 'en' || lang === 'zh-Hant' ? lang : 'zh-Hans'
}

export const useThemeStore = defineStore('theme', () => {
  const themeMode = ref<ThemeMode>('system')
  // 系统主题唯一数据源为后端（初始值经查询、变化经事件），未加载前默认浅色
  const systemIsDark = ref(false)
  const naiveTheme = ref<GlobalTheme | null>(null)
  const locale = ref<Locale>('zh-Hans')

  /** 搜索栏占位符文本（响应式，直接绑定到 SearchBar 的 placeholder 属性） */
  const searchBarPlaceholder = ref('Hello, ZeroLaunch! ヾ(≧▽≦*)o')

  /** 应用 Naive UI 主题并重新计算 CSS 变量（含背景图片异步解析） */
  async function applyNaiveTheme() {
    const dark = themeMode.value === 'dark' || (themeMode.value === 'system' && systemIsDark.value)
    naiveTheme.value = dark ? darkTheme : null
    document.documentElement.classList.toggle('dark', dark)

    // 主题切换时重新应用配色与背景图片 CSS 变量
    if (Object.keys(currentAppearanceSettings).length > 0) {
      await applyAppearanceSettings(currentAppearanceSettings)
    }
  }

  /** 当前内存中的外观配置缓存（用于主题切换时重新应用配色） */
  let currentAppearanceSettings: Record<string, unknown> = {}

  /** 系统主题事件解绑函数：loadFromBackend 重复调用时先解绑旧监听再注册，避免监听器叠加。 */
  let unlistenSystemTheme: (() => void) | null = null

  /** 从后端加载配置（主题 + 语言 + 全部外观设置），在应用挂载前调用。
   *  语言归属 general-config，主题与外观字段归属 appearance-config。 */
  async function loadFromBackend(): Promise<Locale> {
    let lang: Locale = 'zh-Hans'
    try {
      const [appearance, general] = await Promise.all([
        configGetSettings('appearance-config'),
        configGetSettings('general-config'),
      ])
      const s = appearance as Record<string, unknown> | undefined
      const g = general as Record<string, unknown> | undefined
      const t = (s?.theme as ThemeMode | undefined) ?? 'system'
      themeMode.value = t
      lang = normalizeLocale(g?.language)
      locale.value = lang

      // 应用外观 CSS 变量并同步响应式状态
      if (s) {
        currentAppearanceSettings = s
        await applyAppearanceSettings(s)
        searchBarPlaceholder.value = extractPlaceholder(s)
      }
    } catch {
      console.warn('[theme-store] Failed to load config, using defaults')
      themeMode.value = 'system'
    }

    // 系统主题唯一数据源：初始值经后端查询（后端读注册表 AppsUseLightTheme），
    // 运行期变化经 system-theme-changed 事件推送（后端注册表监听驱动）。
    // 先注册事件监听再查询：查询窗口期内的事件不丢失，查询值（最新系统值）覆盖语义一致。
    // loadFromBackend 可能被重复调用（多窗口/热更新），先解绑旧监听避免叠加。
    if (unlistenSystemTheme) {
      unlistenSystemTheme()
    }
    unlistenSystemTheme = await onSystemThemeChanged((isDark) => {
      systemIsDark.value = isDark
      if (themeMode.value === 'system') {
        applyNaiveTheme()
      }
    })

    try {
      systemIsDark.value = (await bridgeGetSystemTheme()) === 'dark'
    } catch {
      // 查询失败保持当前值，事件到达后修正
    }

    await applyNaiveTheme()

    return lang
  }

  /** 应用跨窗口同步的外观配置（主题 + 外观字段；语言已归属 general-config） */
  async function applyRemoteAppearance(settings: Record<string, unknown>) {
    const t = (settings.theme as ThemeMode | undefined) ?? themeMode.value
    const themeChanged = t !== themeMode.value

    // 先更新缓存，再应用主题，确保 applyNaiveTheme 使用的是最新配置
    currentAppearanceSettings = settings

    if (themeChanged) {
      themeMode.value = t
      await applyNaiveTheme()
    }

    // 重新应用外观 CSS 变量并同步响应式状态（即使主题未变，配置字段也可能有变化）
    if (!themeChanged) {
      await applyAppearanceSettings(settings)
    }
    searchBarPlaceholder.value = extractPlaceholder(settings)

    return { themeChanged }
  }

  /** 应用跨窗口同步的常规配置（语言），返回语言是否变化 */
  async function applyRemoteGeneral(settings: Record<string, unknown>) {
    const l = normalizeLocale(settings.language)
    const langChanged = l !== locale.value
    if (langChanged) {
      locale.value = l
    }
    return { langChanged, newLang: l }
  }

  return {
    themeMode,
    naiveTheme,
    locale,
    searchBarPlaceholder,
    loadFromBackend,
    applyRemoteAppearance,
    applyRemoteGeneral,
  }
})
