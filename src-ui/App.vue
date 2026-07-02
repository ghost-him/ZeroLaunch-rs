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
import { i18n } from './i18n'
import { onConfigChanged } from './bridge/events'

const themeStore = useThemeStore()
const configStore = useConfigStore()

const naiveLocale = ref(i18n.global.locale.value === 'en' ? enUS : zhCN)

let unlistenAppearance: (() => void) | null = null
let unlistenWindowBehavior: (() => void) | null = null

onMounted(async () => {
  // 监听外观配置变更（跨窗口同步主题/语言/外观CSS变量）
  unlistenAppearance = await onConfigChanged((payload) => {
    if (payload.componentId !== 'appearance-config') return
    configGetSettings('appearance-config').then(async (s) => {
      const settings = s as Record<string, unknown>
      const result = await themeStore.applyRemoteSettings(settings)
      if (result.langChanged) {
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
  unlistenWindowBehavior?.()
})
</script>
