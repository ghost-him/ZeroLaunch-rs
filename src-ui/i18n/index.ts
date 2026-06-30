import { createI18n } from 'vue-i18n'
import zhHans from './locales/zh-Hans.json'
import en from './locales/en.json'

export type Locale = 'zh-Hans' | 'en'

export const i18n = createI18n({
  legacy: false,
  locale: 'zh-Hans',
  fallbackLocale: 'en',
  messages: {
    'zh-Hans': zhHans,
    en,
  },
})

export function getInitialLocale(): Locale {
  // Will be overridden after backend config loads; system language as fallback
  try {
    const navLang = navigator.language
    if (navLang.startsWith('zh')) return 'zh-Hans'
    return 'en'
  } catch {
    return 'en'
  }
}

export function setLocale(locale: Locale) {
  i18n.global.locale.value = locale
}
