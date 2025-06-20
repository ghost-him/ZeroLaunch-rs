// src/router.ts
import { createRouter, createWebHistory } from 'vue-router';
import App from './views/App.vue';
import SettingWindow from './views/SettingWindow.vue';
import Welcome from './views/welcome.vue';

const routes = [
  { path: '/', component: App },
  { path: '/setting_window', component: SettingWindow },
  { path: '/welcome', component: Welcome },
];

const router = createRouter({
  history: createWebHistory(),
  routes,
});

export default router;