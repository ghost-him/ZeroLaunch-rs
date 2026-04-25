use crate::core::config::ConfigManager;
use crate::error::OptionExt;
use crate::plugin_system::service::PluginService;
use crate::plugin_system::DefaultPluginAPI;
use crate::plugin_system::SessionRouter;
use crate::sdk::HostApi;
use crate::utils::waiting_hashmap::AsyncWaitingHashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use tauri::menu::Menu;
use tauri::tray::TrayIcon;
use tauri::AppHandle;

pub struct AppState {
    session_router: Arc<SessionRouter>,
    config_manager: RwLock<Option<Arc<ConfigManager>>>,
    main_handle: RwLock<Option<Arc<AppHandle>>>,
    is_search_bar_visible: RwLock<bool>,
    waiting_hashmap: Arc<AsyncWaitingHashMap<String, Vec<(String, String)>>>,
    tray_icon: RwLock<Option<Arc<TrayIcon>>>,
    tray_menu: RwLock<Option<Arc<Menu<tauri::Wry>>>>,
    game_mode: RwLock<bool>,
    is_keyboard_blocked: RwLock<bool>,
    previous_foreground_window: RwLock<Option<isize>>,
    previous_selection: RwLock<Option<String>>,
    host_api: RwLock<Option<Arc<HostApi>>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        let api = Arc::new(DefaultPluginAPI::new());
        let plugin_service = Arc::new(PluginService::new(api));
        let session_router = Arc::new(SessionRouter::new(plugin_service));

        AppState {
            session_router,
            config_manager: RwLock::new(None),
            main_handle: RwLock::new(None),
            is_search_bar_visible: RwLock::new(false),
            waiting_hashmap: Arc::new(AsyncWaitingHashMap::new()),
            tray_icon: RwLock::new(None),
            tray_menu: RwLock::new(None),
            game_mode: RwLock::new(false),
            is_keyboard_blocked: RwLock::new(false),
            previous_foreground_window: RwLock::new(None),
            previous_selection: RwLock::new(None),
            host_api: RwLock::new(None),
        }
    }

    pub fn get_session_router(&self) -> &Arc<SessionRouter> {
        &self.session_router
    }

    pub fn get_config_manager(&self) -> Arc<ConfigManager> {
        self.config_manager
            .read()
            .as_ref()
            .cloned()
            .expect_programming("config manager not initialized")
    }

    pub fn set_config_manager(&self, config_manager: Arc<ConfigManager>) {
        *self.config_manager.write() = Some(config_manager);
    }

    pub fn get_main_handle(&self) -> Arc<AppHandle> {
        self.main_handle
            .read()
            .as_ref()
            .cloned()
            .expect_programming("main handle not initialized")
    }

    pub fn set_main_handle(&self, handle: Arc<AppHandle>) {
        *self.main_handle.write() = Some(handle);
    }

    pub fn set_search_bar_visible(&self, is_visible: bool) {
        *self.is_search_bar_visible.write() = is_visible;
    }

    pub fn get_search_bar_visible(&self) -> bool {
        *self.is_search_bar_visible.read()
    }

    pub fn get_waiting_hashmap(&self) -> Arc<AsyncWaitingHashMap<String, Vec<(String, String)>>> {
        self.waiting_hashmap.clone()
    }

    pub fn set_tray_icon(&self, client: Arc<TrayIcon>) {
        *self.tray_icon.write() = Some(client);
    }

    pub fn get_tray_icon(&self) -> Arc<TrayIcon> {
        self.tray_icon
            .read()
            .as_ref()
            .cloned()
            .expect_programming("tray icon not initialized")
    }

    pub fn set_tray_menu(&self, menu: Arc<Menu<tauri::Wry>>) {
        *self.tray_menu.write() = Some(menu);
    }

    pub fn get_tray_menu(&self) -> Arc<Menu<tauri::Wry>> {
        self.tray_menu
            .read()
            .as_ref()
            .cloned()
            .expect_programming("tray menu not initialized")
    }

    pub fn set_game_mode(&self, game_mode: bool) {
        *self.game_mode.write() = game_mode;
    }

    pub fn get_game_mode(&self) -> bool {
        *self.game_mode.read()
    }

    pub fn set_is_keyboard_blocked(&self, is_keyboard_blocked: bool) {
        *self.is_keyboard_blocked.write() = is_keyboard_blocked;
    }

    pub fn get_is_keyboard_blocked(&self) -> bool {
        *self.is_keyboard_blocked.read()
    }

    pub fn set_previous_foreground_window(&self, hwnd: Option<isize>) {
        *self.previous_foreground_window.write() = hwnd;
    }

    pub fn get_previous_foreground_window(&self) -> Option<isize> {
        *self.previous_foreground_window.read()
    }

    pub fn set_previous_selection(&self, selection: Option<String>) {
        *self.previous_selection.write() = selection;
    }

    pub fn get_previous_selection(&self) -> Option<String> {
        self.previous_selection.read().clone()
    }

    pub fn get_host_api(&self) -> Arc<HostApi> {
        self.host_api
            .read()
            .as_ref()
            .cloned()
            .expect_programming("host_api not initialized")
    }

    pub fn set_host_api(&self, host_api: Arc<HostApi>) {
        *self.host_api.write() = Some(host_api);
    }
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("main_handle", &self.main_handle)
            .field("waiting_hashmap", &self.waiting_hashmap)
            .finish()
    }
}
