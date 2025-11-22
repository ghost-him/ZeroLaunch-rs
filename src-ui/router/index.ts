import { createRouter, createWebHistory } from 'vue-router';
import Home from '../windows/Home.vue';
import Welcome from '../windows/Welcome.vue';

// Settings Layout
import SettingsLayout from '../windows/SettingsLayout.vue';

// Settings Sub-views
import General from '../windows/settings/general/General.vue';
import ConfigPath from '../windows/settings/ConfigPath.vue';
import Shortcuts from '../windows/settings/Shortcuts.vue';
import About from '../windows/settings/About.vue';
import Debug from '../windows/settings/Debug.vue';

// Appearance Settings
import SearchStyle from '../windows/settings/appearance/SearchStyle.vue';
import Background from '../windows/settings/appearance/Background.vue';
import Window from '../windows/settings/appearance/Window.vue';

// Program Settings
import ProgramPaths from '../windows/settings/programs/Paths.vue';
import ProgramBlocklist from '../windows/settings/programs/Blocklist.vue';
import ProgramKeywords from '../windows/settings/programs/Keywords.vue';
import ProgramAliases from '../windows/settings/programs/Aliases.vue';
import ProgramAdvanced from '../windows/settings/programs/Advanced.vue';

// Search Settings
import WebSearch from '../windows/settings/search/WebSearch.vue';
import CustomCommands from '../windows/settings/search/CustomCommands.vue';
import BuiltinCommands from '../windows/settings/search/BuiltinCommands.vue';

const routes = [
  { path: '/', component: Home },
  { 
    path: '/setting_window', 
    component: SettingsLayout,
    children: [
      { path: '', redirect: '/setting_window/general' },
      { path: 'general', component: General },
      { path: 'appearance/search', component: SearchStyle },
      { path: 'appearance/background', component: Background },
      { path: 'appearance/window', component: Window },
      { path: 'programs/paths', component: ProgramPaths },
      { path: 'programs/blocklist', component: ProgramBlocklist },
      { path: 'programs/keywords', component: ProgramKeywords },
      { path: 'programs/aliases', component: ProgramAliases },
      { path: 'programs/advanced', component: ProgramAdvanced },
      { path: 'search/web', component: WebSearch },
      { path: 'search/custom', component: CustomCommands },
      { path: 'search/builtin', component: BuiltinCommands },
      { path: 'config', component: ConfigPath },
      { path: 'shortcuts', component: Shortcuts },
      { path: 'about', component: About },
      { path: 'debug', component: Debug },
    ]
  },
  { path: '/welcome', component: Welcome },
];

const router = createRouter({
  history: createWebHistory(),
  routes,
});

export default router;
