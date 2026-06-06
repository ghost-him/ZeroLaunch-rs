use crate::cli_server::token::CliToken;
use crate::core::config::ConfigManager;
use crate::core::tray::TrayManager;
use crate::plugin_system::service::PluginService;
use crate::plugin_system::SessionRouter;
use crate::sdk::HostApi;
use crate::utils::waiting_hashmap::AsyncWaitingHashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use tauri::AppHandle;
use zerolaunch_plugin_api::host::PluginHandle;
use zerolaunch_plugin_host::manager::PluginHostManager;

pub struct AppState {
    session_router: Arc<SessionRouter>,
    config_manager: RwLock<Option<Arc<ConfigManager>>>,
    main_handle: RwLock<Option<Arc<AppHandle>>>,
    waiting_hashmap: Arc<AsyncWaitingHashMap<String, Vec<(String, String)>>>,
    tray_manager: RwLock<Option<Arc<TrayManager>>>,
    game_mode: RwLock<bool>,
    is_keyboard_blocked: RwLock<bool>,
    previous_foreground_window: RwLock<Option<isize>>,
    previous_selection: RwLock<Option<String>>,
    host_api: RwLock<Option<Arc<HostApi>>>,
    core_handle: RwLock<Option<Arc<PluginHandle>>>,
    #[cfg(feature = "inspector")]
    inspector: RwLock<Option<Arc<crate::plugin_system::inspector::Inspector>>>,
    plugin_host_manager: RwLock<Option<Arc<PluginHostManager>>>,
    /// CLI server token (cached for the `cli_get_info` IPC command).
    cli_token: RwLock<Option<CliToken>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        let plugin_service = Arc::new(PluginService::new());
        let session_router = Arc::new(SessionRouter::new(plugin_service));

        AppState {
            session_router,
            config_manager: RwLock::new(None),
            main_handle: RwLock::new(None),
            waiting_hashmap: Arc::new(AsyncWaitingHashMap::new()),
            tray_manager: RwLock::new(None),
            game_mode: RwLock::new(false),
            is_keyboard_blocked: RwLock::new(false),
            previous_foreground_window: RwLock::new(None),
            previous_selection: RwLock::new(None),
            host_api: RwLock::new(None),
            core_handle: RwLock::new(None),
            #[cfg(feature = "inspector")]
            inspector: RwLock::new(None),
            plugin_host_manager: RwLock::new(None),
            cli_token: RwLock::new(None),
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
            .expect("config manager not initialized")
    }

    pub fn set_config_manager(&self, config_manager: Arc<ConfigManager>) {
        *self.config_manager.write() = Some(config_manager);
    }

    pub fn get_main_handle(&self) -> Arc<AppHandle> {
        self.main_handle
            .read()
            .as_ref()
            .cloned()
            .expect("main handle not initialized")
    }

    pub fn set_main_handle(&self, handle: Arc<AppHandle>) {
        *self.main_handle.write() = Some(handle);
    }

    pub fn get_waiting_hashmap(&self) -> Arc<AsyncWaitingHashMap<String, Vec<(String, String)>>> {
        self.waiting_hashmap.clone()
    }

    pub fn set_tray_manager(&self, manager: Arc<TrayManager>) {
        *self.tray_manager.write() = Some(manager);
    }

    pub fn get_tray_manager(&self) -> Option<Arc<TrayManager>> {
        self.tray_manager.read().clone()
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
            .expect("host_api not initialized")
    }

    pub fn set_host_api(&self, host_api: Arc<HostApi>) {
        *self.host_api.write() = Some(host_api);
    }

    pub fn get_core_handle(&self) -> Arc<PluginHandle> {
        self.core_handle
            .read()
            .as_ref()
            .cloned()
            .expect("core_handle not initialized")
    }

    pub fn set_core_handle(&self, handle: Arc<PluginHandle>) {
        *self.core_handle.write() = Some(handle);
    }

    #[cfg(feature = "inspector")]
    pub fn get_inspector(&self) -> Option<Arc<crate::plugin_system::inspector::Inspector>> {
        self.inspector.read().clone()
    }

    #[cfg(feature = "inspector")]
    pub fn set_inspector(&self, inspector: Arc<crate::plugin_system::inspector::Inspector>) {
        *self.inspector.write() = Some(inspector);
    }

    pub fn get_plugin_host_manager(&self) -> Option<Arc<PluginHostManager>> {
        self.plugin_host_manager.read().clone()
    }

    pub fn set_plugin_host_manager(&self, manager: Arc<PluginHostManager>) {
        *self.plugin_host_manager.write() = Some(manager);
    }

    pub fn get_cli_token(&self) -> Option<CliToken> {
        self.cli_token.read().clone()
    }

    pub fn set_cli_token(&self, token: CliToken) {
        *self.cli_token.write() = Some(token);
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
