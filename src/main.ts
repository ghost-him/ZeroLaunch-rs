import { createApp } from "vue";
import ElementPlus from 'element-plus';
import 'element-plus/dist/index.css';
import router from './router.ts';
import Router from "./Router.vue";

const app = createApp(Router);
app.use(ElementPlus);
app.use(router);
app.mount("#app");
