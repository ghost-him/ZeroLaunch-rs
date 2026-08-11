<template>
  <n-config-provider :theme="themeStore.naiveTheme" :locale="naiveLocale">
    <n-notification-provider>
      <n-message-provider>
        <n-dialog-provider>
          <router-view />
        </n-dialog-provider>
      </n-message-provider>
    </n-notification-provider>
  </n-config-provider>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import {
  NConfigProvider,
  NNotificationProvider,
  NMessageProvider,
  NDialogProvider,
  zhCN,
  enUS,
} from 'naive-ui'
import { useThemeStore } from './stores/theme-store'
import { useConfigStore } from './stores/config-store'
import { configGetSettings } from './bridge/commands'
import { i18n, setLocale, type Locale } from './i18n'
import { refreshPluginTranslations } from './stores/i18n-store'
import { onConfigChanged, onPluginInstalled, onPluginUninstalled } from './bridge/events'

const themeStore = useThemeStore()
const configStore = useConfigStore()

const naiveLocale = ref(i18n.global.locale.value === 'en' ? enUS : zhCN)

let unlistenAppearance: (() => void) | null = null
let unlistenGeneral: (() => void) | null = null
let unlistenWindowBehavior: (() => void) | null = null
let unlistenPluginEvents: (() => void)[] = []

onMounted(async () => {
  // 拉取第三方插件翻译目录（内置语言包已静态打包）
  refreshPluginTranslations(i18n.global.locale.value as Locale)
  // 插件安装/卸载后刷新合并目录（跨窗口：设置窗口安装的插件主窗口也要生效）
  unlistenPluginEvents = [
    await onPluginInstalled(() => refreshPluginTranslations(i18n.global.locale.value as Locale)),
    await onPluginUninstalled(() => refreshPluginTranslations(i18n.global.locale.value as Locale)),
  ]
  // 监听外观配置变更（跨窗口同步主题/外观CSS变量）
  unlistenAppearance = await onConfigChanged((payload) => {
    if (payload.componentId !== 'appearance-config') return
    configGetSettings('appearance-config').then(async (s) => {
      await themeStore.applyRemoteAppearance(s as Record<string, unknown>)
    }).catch(() => {})
  })

  // 监听常规配置变更（跨窗口同步界面语言）
  unlistenGeneral = await onConfigChanged((payload) => {
    if (payload.componentId !== 'general-config') return
    configGetSettings('general-config').then(async (s) => {
      const result = await themeStore.applyRemoteGeneral(s as Record<string, unknown>)
      if (result.langChanged) {
        setLocale(result.newLang)
        refreshPluginTranslations(result.newLang)
        naiveLocale.value = result.newLang === 'en' ? enUS : zhCN
      }
    }).catch(() => {})
  })

  // 加载窗口行为配置（供 useKeyboard 消费）
  configGetSettings('window-behavior-config').then(s => {
    configStore.settings['window-behavior-config'] = s
  }).catch(() => {})

  // 监听窗口行为配置变更（跨窗口同步）
  unlistenWindowBehavior = await onConfigChanged((payload) => {
    if (payload.componentId !== 'window-behavior-config') return
    configGetSettings('window-behavior-config').then(s => {
      configStore.settings['window-behavior-config'] = s
    }).catch(() => {})
  })
})

onUnmounted(() => {
  unlistenAppearance?.()
  unlistenGeneral?.()
  unlistenWindowBehavior?.()
  unlistenPluginEvents.forEach((fn) => fn())
})
</script>
