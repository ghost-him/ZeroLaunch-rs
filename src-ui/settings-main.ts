import { createApp } from 'vue'
import { createPinia } from 'pinia'
import SettingsApp from './SettingsApp.vue'
import { i18n, setLocale, type Locale } from './i18n'
import { useThemeStore } from './stores/theme-store'
import { useAssistantStore } from './stores/assistant-store'
import { usePluginManager } from './composables/usePluginManager'
import './styles/variables.css'
import './styles/transitions.css'

async function init() {
  const app = createApp(SettingsApp)
  const pinia = createPinia()
  app.use(pinia)
  app.use(i18n)

  const themeStore = useThemeStore(pinia)
  const lang: Locale = await themeStore.loadFromBackend()
  setLocale(lang)
  // 助手对话在挂载前初始化：跨页面切换保留记录，且不在组件内首次创建 store
  useAssistantStore(pinia)

  app.mount('#app')

  const { loadBuiltinPlugins, loadThirdPartyPlugins } = usePluginManager()
  loadBuiltinPlugins()
  loadThirdPartyPlugins()
}

init().catch((e) => {
  console.error('[settings-main] Failed to initialize app:', e)
})
