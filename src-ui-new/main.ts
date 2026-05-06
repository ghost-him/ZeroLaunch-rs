import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import { router } from './router'
import { usePluginManager } from './composables/usePluginManager'
import './styles/variables.css'
import './styles/transitions.css'

const app = createApp(App)
const pinia = createPinia()
app.use(pinia)
app.use(router)
app.mount('#app')

// 加载内置前端插件
const { loadBuiltinPlugins } = usePluginManager()
loadBuiltinPlugins()
