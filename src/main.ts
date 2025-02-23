import { createApp } from "vue";
import { createPinia } from "pinia";
import ElementPlus from 'element-plus';
import 'element-plus/dist/index.css';
import router from './router.ts';
import Router from "./views/Router.vue";

const app = createApp(Router);
const pinia = createPinia()
app.use(pinia)
app.use(ElementPlus);
app.use(router);
app.mount("#app");
