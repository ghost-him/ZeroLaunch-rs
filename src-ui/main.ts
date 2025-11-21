import { createApp } from "vue";
import { createPinia } from "pinia";
import ElementPlus from 'element-plus';
import 'element-plus/dist/index.css';
import router from './router/index';
import App from "./App.vue";
import i18n from './i18n';

const app = createApp(App);
const pinia = createPinia();

app.use(pinia);
app.use(ElementPlus);
app.use(router);
app.use(i18n);

app.mount("#app");
