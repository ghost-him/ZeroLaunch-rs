// src/router.ts
import { createRouter, createWebHistory } from 'vue-router';
import App from './views/App.vue';
import SettingWindow from './views/SettingWindow.vue';

const routes = [
  { path: '/', component: App },
  { path: '/setting_window', component: SettingWindow },
];

const router = createRouter({
  history: createWebHistory(),
  routes,
});

export default router;