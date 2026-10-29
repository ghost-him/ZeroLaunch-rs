<template>
  <n-config-provider :theme="themeStore.naiveTheme" :locale="naiveLocale">
    <n-notification-provider>
      <n-message-provider>
        <n-dialog-provider>
          <SettingsView />
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
} from 'naive-ui'
import { naiveLocaleFor } from './utils/locale'
import { useThemeStore } from './stores/theme-store'
import { configGetSettings } from './bridge/commands'
import { i18n, setLocale, type Locale } from './i18n'
import { refreshPluginTranslations } from './stores/i18n-store'
import { onConfigChanged } from './bridge/events'
import { useFileDropRouter } from './composables/useFileDrop'
import SettingsView from './views/SettingsView.vue'

const themeStore = useThemeStore()

// 设置窗拖放路由：整窗唯一监听，字段级与页面级投放目标自行注册
useFileDropRouter()

const naiveLocale = ref(naiveLocaleFor(i18n.global.locale.value))

let unlistenAppearance: (() => void) | null = null
let unlistenGeneral: (() => void) | null = null

onMounted(async () => {
  // 拉取第三方插件翻译目录（内置语言包已静态打包）
  refreshPluginTranslations(i18n.global.locale.value as Locale)
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
        naiveLocale.value = naiveLocaleFor(result.newLang)
      }
    }).catch(() => {})
  })
})

onUnmounted(() => {
  unlistenAppearance?.()
  unlistenGeneral?.()
})
</script>
