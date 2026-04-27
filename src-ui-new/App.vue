<template>
  <n-config-provider :theme="themeStore.naiveTheme">
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
import {
  NConfigProvider,
  NNotificationProvider,
  NMessageProvider,
  NDialogProvider,
} from 'naive-ui'
import { useThemeStore } from './stores/theme-store'

const themeStore = useThemeStore()
</script>
