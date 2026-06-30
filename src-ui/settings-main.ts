import { createApp } from 'vue'
import { createPinia } from 'pinia'
import SettingsApp from './SettingsApp.vue'
import { i18n, setLocale, type Locale } from './i18n'
import { useThemeStore } from './stores/theme-store'
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

  app.mount('#app')

  const { loadBuiltinPlugins } = usePluginManager()
  loadBuiltinPlugins()
}

init().catch((e) => {
  console.error('[settings-main] Failed to initialize app:', e)
})
