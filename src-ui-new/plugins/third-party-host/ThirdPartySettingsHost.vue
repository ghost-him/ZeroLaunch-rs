<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue'
import { cliGetInfo } from '@/bridge/commands'
import { isValidPluginMessage } from './postMessageBridge'

const props = defineProps<{
  pluginId: string
  settingsEntryUrl: string
  currentSettings: unknown
}>()

const emit = defineEmits<{
  (e: 'save', settings: unknown): void
}>()

const iframeRef = ref<HTMLIFrameElement | null>(null)

/// Inject CLI connection info into the iframe so it can call host APIs.
async function onIframeLoad() {
  try {
    const info = await cliGetInfo()
    iframeRef.value?.contentWindow?.postMessage(
      { type: 'cli-info', cliInfo: info },
      '*',
    )
  } catch (err) {
    console.error('[ThirdPartySettingsHost] Failed to get CLI info:', err)
  }
}

function onMessage(e: MessageEvent) {
  if (!iframeRef.value || e.source !== iframeRef.value.contentWindow) return
  if (!isValidPluginMessage(e, props.pluginId)) return
  if (e.data?.type === 'save-settings') {
    emit('save', e.data.settings)
  }
}

onMounted(() => window.addEventListener('message', onMessage))
onUnmounted(() => window.removeEventListener('message', onMessage))

watch(
  () => props.currentSettings,
  (newSettings) => {
    iframeRef.value?.contentWindow?.postMessage(
      { type: 'settings-update', settings: newSettings },
      '*',
    )
  },
)

const iframeSrc = `/__zlplugin_iframe__.html?plugin=${encodeURIComponent(props.pluginId)}&entry=${encodeURIComponent(props.settingsEntryUrl)}`
</script>

<template>
  <iframe
    ref="iframeRef"
    :src="iframeSrc"
    sandbox="allow-scripts allow-same-origin"
    style="width: 100%; height: 100%; border: 0"
    @load="onIframeLoad"
  />
</template>
