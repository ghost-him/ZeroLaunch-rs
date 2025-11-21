import { createI18n } from 'vue-i18n';
import en from './locales/en.json';
import zhHans from './locales/zh-Hans.json';
import zhHant from './locales/zh-Hant.json';

const i18n = createI18n({
  legacy: false,
  locale: 'zh-Hans',
  fallbackLocale: 'en',
  messages: {
    en,
    'zh-Hans': zhHans,
    'zh-Hant': zhHant,
  },
});

export const supportedLanguages = ['zh-Hans', 'zh-Hant', 'en'];

export const initializeLanguage = (language: string) => {
  if (language && supportedLanguages.includes(language)) {
    // @ts-ignore
    i18n.global.locale.value = language;
  }
};

export default i18n;
