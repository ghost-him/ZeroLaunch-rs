/**
 * 外观配置工具模块。
 * 将后端 AppearanceConfigComponent 的 JSON 配置映射为 CSS 自定义属性。
 *
 * 设计原则：
 * - 后端是配置值的唯一权威来源，此处不重复定义默认值
 * - 仅对 settings 中实际存在的 key 设置 CSS 变量
 * - 缺失 key 时不修改对应 CSS 变量，由 variables.css 中的静态默认值兜底
 */

import { resourceGet } from '../bridge/commands'

type AppearanceSettings = Record<string, unknown>

/**
 * 将外观配置映射为 CSS 变量并应用到 document.documentElement。
 * 仅设置 settings 中存在的 key；缺失 key 不修改 CSS 变量。
 */
export async function applyAppearanceSettings(settings: AppearanceSettings): Promise<void> {
  const root = document.documentElement.style
  const isDark = document.documentElement.classList.contains('dark')

  // ---- 搜索栏 ----
  if ('search_bar_height' in settings) {
    const h = asNum(settings.search_bar_height)
    root.setProperty('--search-bar-height', h + 'px')
    if ('search_bar_font_ratio' in settings) {
      root.setProperty('--font-size-xl', Math.round(h * asNum(settings.search_bar_font_ratio)) + 'px')
    }
  }
  if ('search_bar_font_family' in settings) {
    root.setProperty('--search-bar-font-family', asStr(settings.search_bar_font_family) || 'inherit')
  }
  if ('search_bar_placeholder' in settings) {
    root.setProperty('--search-bar-placeholder', `"${asStr(settings.search_bar_placeholder)}"`)
  }

  // ---- 结果栏 ----
  if ('result_item_height' in settings) {
    const h = asNum(settings.result_item_height)
    root.setProperty('--result-item-height', h + 'px')
    if ('result_item_font_ratio' in settings) {
      root.setProperty('--font-size-lg', Math.round(h * asNum(settings.result_item_font_ratio)) + 'px')
    }
    if ('result_item_subtitle_font_ratio' in settings) {
      root.setProperty('--font-size-md', Math.round(h * asNum(settings.result_item_subtitle_font_ratio)) + 'px')
    }
    if ('result_item_icon_ratio' in settings) {
      root.setProperty('--result-item-icon-size', Math.round(h * asNum(settings.result_item_icon_ratio)) + 'px')
    }
  }
  if ('result_item_font_family' in settings) {
    root.setProperty('--result-item-font-family', asStr(settings.result_item_font_family) || 'inherit')
  }
  if ('max_visible_results' in settings) {
    root.setProperty('--max-visible-results', String(Math.round(asNum(settings.max_visible_results))))
  }

  // ---- 底栏 ----
  if ('footer_height' in settings) {
    const h = asNum(settings.footer_height)
    root.setProperty('--footer-height', h + 'px')
    if ('footer_font_ratio' in settings) {
      root.setProperty('--font-size-sm', Math.round(h * asNum(settings.footer_font_ratio)) + 'px')
    }
  }
  if ('footer_font_family' in settings) {
    root.setProperty('--footer-font-family', asStr(settings.footer_font_family) || 'inherit')
  }

  // ---- 窗口 ----
  if ('window_width' in settings) {
    root.setProperty('--window-width', asNum(settings.window_width) + 'px')
  }
  if ('window_corner_radius' in settings) {
    root.setProperty('--window-corner-radius', asNum(settings.window_corner_radius) + 'px')
  }
  if ('vertical_position_ratio' in settings) {
    root.setProperty('--vertical-position-ratio', String(asNum(settings.vertical_position_ratio)))
  }

  // ---- 配色（深色/浅色整组切换） ----
  if ('bg_primary' in settings) {
    applyColorScheme(root, settings, isDark)
  }

  // ---- 背景图片 ----
  if ('bg_size' in settings) {
    root.setProperty('--bg-size', asStr(settings.bg_size))
  }
  if ('bg_position' in settings) {
    root.setProperty('--bg-position', asStr(settings.bg_position))
  }
  if ('bg_repeat' in settings) {
    root.setProperty('--bg-repeat', asStr(settings.bg_repeat))
  }
  if ('bg_opacity' in settings) {
    root.setProperty('--bg-image-opacity', String(asNum(settings.bg_opacity, 1)))
  }

  // 异步解析背景图片 res:// 标识为 base64 data URL
  // 深色模式下优先使用 bgImageDark，未设置则回退到 bgImage
  if ('bg_image' in settings || 'bg_image_dark' in settings) {
    const darkBg = asStr(settings.bg_image_dark)
    const lightBg = asStr(settings.bg_image)
    const activeBg = isDark && darkBg ? darkBg : lightBg
    const url = await resolveBgUrl(activeBg)
    root.setProperty('--bg-image-url', url)
  }
}

/** 提取搜索栏占位符文本（用于响应式绑定） */
export function extractPlaceholder(settings: AppearanceSettings): string {
  if ('search_bar_placeholder' in settings) {
    return asStr(settings.search_bar_placeholder) || 'Hello, ZeroLaunch! ヾ(≧▽≦*)o'
  }
  // 从 CSS 变量回退读取
  const css = getComputedStyle(document.documentElement).getPropertyValue('--search-bar-placeholder').trim()
  return css.replace(/^["']|["']$/g, '') || 'Hello, ZeroLaunch! ヾ(≧▽≦*)o'
}

// ---- 内部工具 ----

/** 将 res:// 资源标识解析为 CSS url() 值；空字符串返回 "none" */
async function resolveBgUrl(resId: string): Promise<string> {
  if (!resId || !resId.startsWith('res://')) return 'none'
  try {
    const dataUrl = await resourceGet(resId.slice(6))
    return `url(${dataUrl})`
  } catch {
    console.warn('[appearance] Failed to resolve background image:', resId)
    return 'none'
  }
}

function applyColorScheme(root: CSSStyleDeclaration, s: AppearanceSettings, isDark: boolean): void {
  if (isDark) {
    root.setProperty('--bg-primary', asStr(s.dark_bg_primary))
    root.setProperty('--bg-secondary', asStr(s.dark_bg_secondary))
    root.setProperty('--text-primary', asStr(s.dark_text_primary))
    root.setProperty('--text-secondary', asStr(s.dark_text_secondary))
    root.setProperty('--border-color', asStr(s.dark_border_color))
    root.setProperty('--accent-color', asStr(s.dark_accent_color))
    root.setProperty('--hover-color', asStr(s.dark_hover_color))

    const accent = asStr(s.dark_accent_color)
    root.setProperty('--primary-color', accent)
    root.setProperty('--primary-color-alpha', colorToAlpha(accent, 0.15))

    root.setProperty('--bg-color', asStr(s.dark_bg_primary))
    root.setProperty('--bg-color-secondary', asStr(s.dark_bg_secondary))
    root.setProperty('--text-color', asStr(s.dark_text_primary))
    root.setProperty('--text-color-secondary', asStr(s.dark_text_secondary))
  } else {
    root.setProperty('--bg-primary', asStr(s.bg_primary))
    root.setProperty('--bg-secondary', asStr(s.bg_secondary))
    root.setProperty('--text-primary', asStr(s.text_primary))
    root.setProperty('--text-secondary', asStr(s.text_secondary))
    root.setProperty('--border-color', asStr(s.border_color))
    root.setProperty('--accent-color', asStr(s.accent_color))
    root.setProperty('--hover-color', asStr(s.hover_color))

    const accent = asStr(s.accent_color)
    root.setProperty('--primary-color', accent)
    root.setProperty('--primary-color-alpha', colorToAlpha(accent, 0.1))

    root.setProperty('--bg-color', asStr(s.bg_primary))
    root.setProperty('--bg-color-secondary', asStr(s.bg_secondary))
    root.setProperty('--text-color', asStr(s.text_primary))
    root.setProperty('--text-color-secondary', asStr(s.text_secondary))
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
