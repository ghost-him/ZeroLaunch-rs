<template>
  <div class="plugin-settings-host">
    <component
      :is="settingsComponent"
      v-if="settingsComponent"
      :current-settings="currentSettings"
      @save="onSave"
    />
    <n-text v-else depth="3">此组件使用默认表单</n-text>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { NText } from 'naive-ui'
import { usePluginStore } from '@/stores/plugin-store'
import { useConfigStore } from '@/stores/config-store'

const props = defineProps<{
  componentId: string
}>()

const pluginStore = usePluginStore()
const configStore = useConfigStore()

const settingsComponent = computed(() =>
  pluginStore.getSettingsComponent(props.componentId),
)

const currentSettings = computed(() =>
  configStore.settings[props.componentId] ?? null,
)

function onSave(settings: unknown) {
  configStore.applySettings(props.componentId, settings)
}
</script>

<style scoped>
.plugin-settings-host {
  padding: 16px;
}
</style>
