import { createApp } from 'vue'
import { createPinia } from 'pinia'
import SettingsApp from './SettingsApp.vue'
import { usePluginManager } from './composables/usePluginManager'
import './styles/variables.css'
import './styles/transitions.css'

const app = createApp(SettingsApp)
const pinia = createPinia()
app.use(pinia)
app.mount('#app')

// 加载内置前端插件
const { loadBuiltinPlugins } = usePluginManager()
loadBuiltinPlugins()
