import { createI18n } from 'vue-i18n'
import zh from './locales/zh.json'
import en from './locales/en.json'

// 1. 将 messages 定义在一个常量中，以便提取其类型
const messages = {
  zh,
  en
}

// 2. 自动从 messages 的 key 中创建语言联合类型
//    这里 Language 的类型会自动变为 'zh' | 'en'
export type Language = keyof typeof messages

// 定义一个包含所有可用语言的数组，方便进行校验
export const supportedLanguages: Language[] = ['zh', 'en']

const i18n = createI18n({
  legacy: false,
  locale: 'zh',
  fallbackLocale: 'zh',
  messages, // 直接使用上面定义的常量
  globalInjection: true
})

export default i18n

// 初始化语言设置，从配置中读取
export const initializeLanguage = async (language: string) => {
  try {
    if (language && (supportedLanguages as string[]).includes(language)) {
      const configLanguage = language as Language
      if (configLanguage !== i18n.global.locale.value) {
        console.log("成功设置语言: ", configLanguage);
        i18n.global.locale.value = configLanguage
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