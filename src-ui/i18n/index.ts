import { createI18n } from 'vue-i18n'
import zhHans from './locales/zh-Hans.json'
import zhHant from './locales/zh-Hant.json'
import en from './locales/en.json'

export type Locale = 'zh-Hans' | 'zh-Hant' | 'en'

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
