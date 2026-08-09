import { createI18n } from 'vue-i18n'
import zhHans from './locales/zh-Hans.json'
import zhHant from './locales/zh-Hant.json'
import en from './locales/en.json'

export type Locale = 'zh-Hans' | 'zh-Hant' | 'en'

/** 内置语言包原始内容（全量重建插件合并时作为 base，避免 merge 累积残留）。 */
export const baseMessages: Record<Locale, Record<string, unknown>> = {
  'zh-Hans': zhHans as unknown as Record<string, unknown>,
  'zh-Hant': zhHant as unknown as Record<string, unknown>,
  en: en as unknown as Record<string, unknown>,
}

export const i18n = createI18n({
  legacy: false,
  locale: 'zh-Hans',
  fallbackLocale: 'en',
  messages: {
    'zh-Hans': zhHans,
    'zh-Hant': zhHant,
    en,
  },
})

/**
 * key-or-literal 渲染：字符串命中翻译目录则返回译文，否则原样显示。
 * 用于处理后端下发的 schema 标签/组件名/动作标签等可能携带 i18n key 的文本
 * （未迁移/第三方字面量直接透传）。
 */
export function resolveText(s: string): string {
  return i18n.global.te(s) ? i18n.global.t(s) : s
}

export function getInitialLocale(): Locale {
  // Will be overridden after backend config loads; system language as fallback
  try {
    const navLang = navigator.language
    if (navLang.startsWith('zh')) {
      // 繁体区域（台湾/港澳等）默认繁体中文，其余中文区域默认简体
      return navLang.toLowerCase().startsWith('zh-tw') ||
        navLang.toLowerCase().startsWith('zh-hk') ||
        navLang.toLowerCase().startsWith('zh-mo') ||
        navLang.toLowerCase().startsWith('zh-hant')
        ? 'zh-Hant'
        : 'zh-Hans'
    }
    return 'en'
  } catch {
    return 'en'
  }
}

export function setLocale(locale: Locale) {
  i18n.global.locale.value = locale
}
