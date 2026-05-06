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
  useNotification,
  zhCN,
  enUS,
} from 'naive-ui'
import { useThemeStore } from './stores/theme-store'
import { registerErrorHandler, configGetSettings } from './bridge/commands'
import type { BridgeError } from './bridge/commands'
import { i18n } from './i18n'
import { onConfigChanged } from './bridge/events'
import SettingsView from './views/SettingsView.vue'

const themeStore = useThemeStore()
const notification = useNotification()

const naiveLocale = ref(i18n.global.locale.value === 'en' ? enUS : zhCN)

registerErrorHandler((error: BridgeError) => {
  notification.error({
    title: error.code,
    content: error.message,
    duration: 5000,
  })
})

let unlistenAppearance: (() => void) | null = null

onMounted(async () => {
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
