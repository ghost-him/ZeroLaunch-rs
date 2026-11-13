import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import { router } from './router'
import { i18n, setLocale, type Locale } from './i18n'
import { useThemeStore } from './stores/theme-store'
import { usePluginManager } from './composables/usePluginManager'
import { installFrontendErrorLogger } from './utils/frontend-logger'
import './styles/variables.css'
import './styles/transitions.css'

/** 前端错误上报入口：release 下 WebView2 控制台不可见，异常统一落后端日志。 */
const reportFrontendIssue = installFrontendErrorLogger('search-window')

async function init() {
  const app = createApp(App)
  const pinia = createPinia()
  app.use(pinia)
  app.use(router)
  app.use(i18n)

  // 挂载前从后端加载外观配置（主题 + 语言），避免渲染闪烁
  const themeStore = useThemeStore(pinia)
  const lang: Locale = await themeStore.loadFromBackend()
  setLocale(lang)

  app.mount('#app')

  // 加载内置前端插件
  const { loadBuiltinPlugins, loadThirdPartyPlugins } = usePluginManager()
  loadBuiltinPlugins()
  loadThirdPartyPlugins()
}

init().catch((e) => {
  console.error('[main] Failed to initialize app:', e)
  reportFrontendIssue(
    `初始化失败: ${e instanceof Error ? e.message : String(e)}`,
    e instanceof Error ? e.stack : undefined,
  )
})
