/**
 * 外观配置工具模块。
 * 将后端 AppearanceConfigComponent 的 JSON 配置映射为 CSS 自定义属性。
 *
 * 设计原则：
 * - 后端是配置值的唯一权威来源，此处不重复定义默认值
 * - 仅对 settings 中实际存在的 key 设置 CSS 变量
 * - 缺失 key 时不修改对应 CSS 变量，由 variables.css 中的静态默认值兜底
 */

type AppearanceSettings = Record<string, unknown>

/**
 * 将外观配置映射为 CSS 变量并应用到 document.documentElement。
 * 仅设置 settings 中存在的 key；缺失 key 不修改 CSS 变量。
 */
export function applyAppearanceSettings(settings: AppearanceSettings): void {
  const root = document.documentElement.style
  const isDark = document.documentElement.classList.contains('dark')

  // ---- 搜索栏 ----
  if ('searchBarHeight' in settings) {
    const h = asNum(settings.searchBarHeight)
    root.setProperty('--search-bar-height', h + 'px')
    if ('searchBarFontRatio' in settings) {
      root.setProperty('--font-size-xl', Math.round(h * asNum(settings.searchBarFontRatio)) + 'px')
    }
  }
  if ('searchBarFontFamily' in settings) {
    root.setProperty('--search-bar-font-family', asStr(settings.searchBarFontFamily) || 'inherit')
  }
  if ('searchBarPlaceholder' in settings) {
    root.setProperty('--search-bar-placeholder', `"${asStr(settings.searchBarPlaceholder)}"`)
  }

  // ---- 结果栏 ----
  if ('resultItemHeight' in settings) {
    const h = asNum(settings.resultItemHeight)
    root.setProperty('--result-item-height', h + 'px')
    if ('resultItemFontRatio' in settings) {
      root.setProperty('--font-size-lg', Math.round(h * asNum(settings.resultItemFontRatio)) + 'px')
    }
    if ('resultItemSubtitleFontRatio' in settings) {
      root.setProperty('--font-size-md', Math.round(h * asNum(settings.resultItemSubtitleFontRatio)) + 'px')
    }
  }
  if ('resultItemFontFamily' in settings) {
    root.setProperty('--result-item-font-family', asStr(settings.resultItemFontFamily) || 'inherit')
  }
  if ('maxVisibleResults' in settings) {
    root.setProperty('--max-visible-results', String(Math.round(asNum(settings.maxVisibleResults))))
  }

  // ---- 底栏 ----
  if ('footerHeight' in settings) {
    const h = asNum(settings.footerHeight)
    root.setProperty('--footer-height', h + 'px')
    if ('footerFontRatio' in settings) {
      root.setProperty('--font-size-sm', Math.round(h * asNum(settings.footerFontRatio)) + 'px')
    }
  }
  if ('footerFontFamily' in settings) {
    root.setProperty('--footer-font-family', asStr(settings.footerFontFamily) || 'inherit')
  }

  // ---- 窗口 ----
  if ('windowWidth' in settings) {
    root.setProperty('--window-width', asNum(settings.windowWidth) + 'px')
  }
  if ('windowCornerRadius' in settings) {
    root.setProperty('--window-corner-radius', asNum(settings.windowCornerRadius) + 'px')
  }
  if ('verticalPositionRatio' in settings) {
    root.setProperty('--vertical-position-ratio', String(asNum(settings.verticalPositionRatio)))
  }

  // ---- 配色（深色/浅色整组切换） ----
  if ('bgPrimary' in settings) {
    applyColorScheme(root, settings, isDark)
  }
}

/** 提取搜索栏占位符文本（用于响应式绑定） */
export function extractPlaceholder(settings: AppearanceSettings): string {
  if ('searchBarPlaceholder' in settings) {
    return asStr(settings.searchBarPlaceholder) || 'Hello, ZeroLaunch! ヾ(≧▽≦*)o'
  }
  // 从 CSS 变量回退读取
  const css = getComputedStyle(document.documentElement).getPropertyValue('--search-bar-placeholder').trim()
  return css.replace(/^["']|["']$/g, '') || 'Hello, ZeroLaunch! ヾ(≧▽≦*)o'
}

// ---- 内部工具 ----

function applyColorScheme(root: CSSStyleDeclaration, s: AppearanceSettings, isDark: boolean): void {
  if (isDark) {
    root.setProperty('--bg-primary', asStr(s.darkBgPrimary))
    root.setProperty('--bg-secondary', asStr(s.darkBgSecondary))
    root.setProperty('--text-primary', asStr(s.darkTextPrimary))
    root.setProperty('--text-secondary', asStr(s.darkTextSecondary))
    root.setProperty('--border-color', asStr(s.darkBorderColor))
    root.setProperty('--accent-color', asStr(s.darkAccentColor))
    root.setProperty('--hover-color', asStr(s.darkHoverColor))

    const accent = asStr(s.darkAccentColor)
    root.setProperty('--primary-color', accent)
    root.setProperty('--primary-color-alpha', colorToAlpha(accent, 0.15))

    root.setProperty('--bg-color', asStr(s.darkBgPrimary))
    root.setProperty('--bg-color-secondary', asStr(s.darkBgSecondary))
    root.setProperty('--text-color', asStr(s.darkTextPrimary))
    root.setProperty('--text-color-secondary', asStr(s.darkTextSecondary))
  } else {
    root.setProperty('--bg-primary', asStr(s.bgPrimary))
    root.setProperty('--bg-secondary', asStr(s.bgSecondary))
    root.setProperty('--text-primary', asStr(s.textPrimary))
    root.setProperty('--text-secondary', asStr(s.textSecondary))
    root.setProperty('--border-color', asStr(s.borderColor))
    root.setProperty('--accent-color', asStr(s.accentColor))
    root.setProperty('--hover-color', asStr(s.hoverColor))

    const accent = asStr(s.accentColor)
    root.setProperty('--primary-color', accent)
    root.setProperty('--primary-color-alpha', colorToAlpha(accent, 0.1))

    root.setProperty('--bg-color', asStr(s.bgPrimary))
    root.setProperty('--bg-color-secondary', asStr(s.bgSecondary))
    root.setProperty('--text-color', asStr(s.textPrimary))
    root.setProperty('--text-color-secondary', asStr(s.textSecondary))
  }
}

function asNum(val: unknown, fallback = 0): number {
  if (typeof val === 'number') return val
  if (typeof val === 'string') {
    const n = Number(val)
    return isNaN(n) ? fallback : n
  }
  return fallback
}

function asStr(val: unknown, fallback = ''): string {
  if (typeof val === 'string') return val
  return fallback
}

function colorToAlpha(hex: string, alpha: number): string {
  const rgbaMatch = hex.match(/^rgba?\((\d+),\s*(\d+),\s*(\d+)/)
  if (rgbaMatch) {
    return `rgba(${rgbaMatch[1]}, ${rgbaMatch[2]}, ${rgbaMatch[3]}, ${alpha})`
  }

  const clean = hex.replace('#', '')
  if (clean.length === 3) {
    const r = parseInt(clean[0] + clean[0], 16)
    const g = parseInt(clean[1] + clean[1], 16)
    const b = parseInt(clean[2] + clean[2], 16)
    return `rgba(${r}, ${g}, ${b}, ${alpha})`
  }
  if (clean.length >= 6) {
    const r = parseInt(clean.substring(0, 2), 16)
    const g = parseInt(clean.substring(2, 4), 16)
    const b = parseInt(clean.substring(4, 6), 16)
    return `rgba(${r}, ${g}, ${b}, ${alpha})`
  }

  return `rgba(32, 128, 240, ${alpha})`
}
