<template>
  <div class="component-config-loader" :class="{ 'is-loaded': isLoaded }">
    <div v-if="loading" class="loading-state">
      <n-spin :size="20" />
    </div>
    <div v-else-if="loadErr" class="error-state">
      <n-text type="error">{{ loadErr }}</n-text>
      <n-button size="small" @click="init">{{ $t('settings.retry') }}</n-button>
    </div>
    <component
      v-else-if="settingsComponent && settings"
      :is="settingsComponent"
      :current-settings="settings"
      @save="onThirdPartySave"
    />
    <DynamicForm
      v-else-if="schema && settings"
      :key="component.componentId"
      :schema="schema"
      :current-settings="settings"
      :group-tabs="groupTabs"
      @reload="init"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { NSpin, NText, NButton, useMessage } from 'naive-ui'
import { useI18n } from 'vue-i18n'
import DynamicForm from './DynamicForm.vue'
import { useConfigStore } from '../../stores/config-store'
import { usePluginStore } from '../../stores/plugin-store'
import { onConfigChanged } from '../../bridge/events'
import { configApplySettings } from '../../bridge/commands'
import type { ComponentInfo, ComponentSchema } from '../../bridge/contract'

const props = withDefaults(defineProps<{
  component: ComponentInfo
  /** 组件内按分组渲染为 tab 页（透传给 DynamicForm） */
  groupTabs?: boolean
}>(), {
  groupTabs: false,
})

const configStore = useConfigStore()
const pluginStore = usePluginStore()

const message = useMessage()
const { t } = useI18n()


const loading = ref(true)
const loadErr = ref<string | null>(null)
const schema = ref<ComponentSchema | null>(null)
const settings = ref<Record<string, unknown> | null>(null)

// 第三方插件自定义设置页组件
const settingsComponent = computed(() =>
  pluginStore.getSettingsComponent(props.component.componentId),
)
let unlistenConfig: (() => void) | null = null

/** 已渲染出可配置内容（DynamicForm 或第三方设置页）时撑满父容器，
 *  使 DynamicForm 内部的 .form-groups 在高度受限时能滚动，而不是被父级裁剪。 */
const isLoaded = computed(
  () => !loading.value && !loadErr.value && !!settings.value
    && (!!settingsComponent.value || !!schema.value),
)

/** 第三方/自定义设置面板（如翻译插件）保存：成功后弹提示并重取设置，失败弹错误并展示错误态。 */
async function onThirdPartySave(newSettings: unknown) {
  try {
    await configApplySettings(props.component.componentId, newSettings)
    message.success(t('settings.saveSuccess'))
    await init()
  } catch (e) {
    message.error(t('settings.saveFailed'))
    loadErr.value = String(e)
  }
}

async function init() {
  loading.value = true
  loadErr.value = null
  try {
    // 有自定义设置页时仍拉 settings；schema 仅 DynamicForm 需要
    if (settingsComponent.value) {
      const cfg = await configStore.getSettings(props.component.componentId)
      settings.value = cfg as Record<string, unknown>
      schema.value = null
    } else {
      const [s, cfg] = await Promise.all([
        configStore.getSchema(props.component.componentId),
        configStore.getSettings(props.component.componentId),
      ])
      schema.value = s
      settings.value = cfg as Record<string, unknown>
    }
  } catch (e) {
    loadErr.value = String(e)
  } finally {
    loading.value = false
  }
}


onMounted(async () => {
  await init()
  unlistenConfig = await onConfigChanged((payload) => {
    if (payload.componentId === props.component.componentId) {
      init()
    }
  })
})

onUnmounted(() => {
  unlistenConfig?.()
})
</script>

<style scoped>
.component-config-loader {
  display: flex;
  flex-direction: column;
  padding: 8px 0;
}

.component-config-loader.is-loaded {
  flex: 1;
  min-height: 0;
  padding: 0;
}

.loading-state,
.error-state {
  padding: 16px;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
}
</style>
