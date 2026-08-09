import { i18n, baseMessages, type Locale } from '../i18n'
import { i18nGetPluginTranslations } from '../bridge/commands'

/**
 * 拉取并合并指定语言下的插件翻译目录。
 * 用「内置 base + 插件目录」全量重建该语言包，而非 mergeLocaleMessage 累积合并，
 * 避免插件卸载后残留旧 key（卸载后的插件目录不再出现，天然清除）。
 * 调用时机：窗口启动、语言切换、插件管理页安装/卸载/重载后。
 */
export async function refreshPluginTranslations(lang: Locale): Promise<void> {
  try {
    const pluginCatalog = await i18nGetPluginTranslations(lang)
    i18n.global.setLocaleMessage(lang, {
      ...baseMessages[lang],
      ...pluginCatalog,
    } as never)
  } catch (e) {
    // 插件目录拉取失败不阻塞界面：内置语言包已在静态打包中，仅缺第三方翻译
    console.warn('[i18n] 拉取插件翻译目录失败:', e)
  }
}
