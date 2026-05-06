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
  useNotification,
  zhCN,
  enUS,
} from 'naive-ui'
import { useThemeStore } from './stores/theme-store'
import { registerErrorHandler, configGetSettings } from './bridge/commands'
import type { BridgeError } from './bridge/commands'
import { i18n } from './i18n'
import { onConfigChanged } from './bridge/events'

const themeStore = useThemeStore()
const notification = useNotification()

const naiveLocale = ref(i18n.global.locale.value === 'en' ? enUS : zhCN)

// 全局错误处理器
registerErrorHandler((error: BridgeError) => {
  notification.error({
    title: error.code,
    content: error.message,
    duration: 5000,
  })
})

let unlistenAppearance: (() => void) | null = null

onMounted(async () => {
  // 监听外观配置变更（跨窗口同步主题/语言）
  unlistenAppearance = await onConfigChanged((payload) => {
    if (payload.componentId !== 'appearance') return
    configGetSettings('appearance').then((s) => {
      const settings = s as Record<string, unknown>
      const result = themeStore.applyRemoteSettings(settings)
      if (result.langChanged) {
        naiveLocale.value = result.newLang === 'en' ? enUS : zhCN
      }
    }).catch(() => {})
  })
})

onUnmounted(() => {
  unlistenAppearance?.()
})
</script>
