import { createApp } from 'vue'
import { createPinia } from 'pinia'
import SettingsApp from './SettingsApp.vue'
import './styles/variables.css'
import './styles/transitions.css'

const app = createApp(SettingsApp)
app.use(createPinia())
app.mount('#app')
