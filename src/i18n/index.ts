import { createI18n } from 'vue-i18n'
import { resolveResource } from '@tauri-apps/api/path'
import { invoke } from '@tauri-apps/api/core';
// 定义支持的语言类型
export type Language = 'zh' | 'en'

// 定义一个包含所有可用语言的数组，方便进行校验
export const supportedLanguages: Language[] = ['zh', 'en']

// 动态加载翻译文件的函数
export const loadLocaleMessages = async (locale: Language) => {
  try {
    const resource_path = await resolveResource(`locales/${locale}.json`);
    console.log(resource_path)
    const content = await invoke<string>('command_read_file', {path: resource_path});
    return JSON.parse(content); 
  } catch (error) {
    console.error(`Error loading locale ${locale}:`, error)
    // 返回空对象作为fallback
    return {}
  }
}

// 创建i18n实例，初始时使用空的messages
const i18n = createI18n({
  legacy: false,
  locale: 'zh',
  fallbackLocale: 'zh',
  messages: {
    zh: {},
    en: {}
  },
  globalInjection: true
})

// 异步初始化默认语言
const initializeDefaultLanguage = async () => {
  try {
    const defaultMessages = await loadLocaleMessages('zh')
    i18n.global.setLocaleMessage('zh', defaultMessages)
  } catch (error) {
    console.error('Failed to load default language:', error)
  }
}

// 立即初始化默认语言
initializeDefaultLanguage()

export default i18n

// 初始化语言设置，从配置中读取
export const initializeLanguage = async (language: string) => {
  try {
    if (language && (supportedLanguages as string[]).includes(language)) {
      const configLanguage = language as Language
      if (configLanguage !== i18n.global.locale.value) {
        // 动态加载新语言的翻译文件
        const messages = await loadLocaleMessages(configLanguage)
        i18n.global.setLocaleMessage(configLanguage, messages)
        i18n.global.locale.value = configLanguage as any
        console.log("成功设置语言: ", configLanguage);
      }
    }
  } catch (error) {
    console.warn('Failed to initialize language from config:', error)
  }
}

// 导出获取当前语言的函数
export const getCurrentLanguage = (): Language => {
  // 函数的返回值类型也更精确了
  return i18n.global.locale.value
}