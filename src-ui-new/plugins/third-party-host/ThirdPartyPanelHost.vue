<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue'
import type { ResultAction } from '@/bridge/contract'
import { cliGetInfo } from '@/bridge/commands'
import { isValidPluginMessage } from './postMessageBridge'

const props = defineProps<{
  pluginId: string
  panelEntryUrl: string
  data: unknown
  actions: ResultAction[]
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
    console.error('[ThirdPartyPanelHost] Failed to get CLI info:', err)
  }
}

function onMessage(e: MessageEvent) {
  if (!iframeRef.value || e.source !== iframeRef.value.contentWindow) return
  if (!isValidPluginMessage(e, props.pluginId)) return
  // Forward plugin messages to the parent app
  // In future: dispatch actions, resize, etc.
}

onMounted(() => window.addEventListener('message', onMessage))
onUnmounted(() => window.removeEventListener('message', onMessage))

watch(
  () => [props.data, props.actions],
  () => {
    iframeRef.value?.contentWindow?.postMessage(
      {
        type: 'data-update',
        data: props.data,
        actions: props.actions,
      },
      '*',
    )
  },
  { deep: true },
)

const iframeSrc = `/__zlplugin_iframe__.html?plugin=${encodeURIComponent(props.pluginId)}&entry=${encodeURIComponent(props.panelEntryUrl)}`
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
