use crate::cli_server::token::CliToken;
use crate::core::config::ConfigManager;
use crate::core::tray::TrayManager;
use crate::plugin_manager::manager::PluginManager;
use crate::plugin_system::service::PluginService;
use crate::plugin_system::SessionRouter;
use crate::sdk::HostApi;
use crate::utils::waiting_hashmap::AsyncWaitingHashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use tauri::AppHandle;
use zerolaunch_plugin_api::host::PluginHandle;

pub struct AppState {
    session_router: Arc<SessionRouter>,
    config_manager: RwLock<Option<Arc<ConfigManager>>>,
    main_handle: RwLock<Option<Arc<AppHandle>>>,
    waiting_hashmap: Arc<AsyncWaitingHashMap<String, Vec<(String, String)>>>,
    tray_manager: RwLock<Option<Arc<TrayManager>>>,
    game_mode: RwLock<bool>,
    host_api: RwLock<Option<Arc<HostApi>>>,
    core_handle: RwLock<Option<Arc<PluginHandle>>>,
    #[cfg(feature = "inspector")]
    inspector: RwLock<Option<Arc<crate::plugin_system::inspector::Inspector>>>,
    /// PluginManager — 插件身份与生命周期的统一入口
    plugin_manager: RwLock<Option<Arc<PluginManager>>>,
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
            host_api: RwLock::new(None),
            core_handle: RwLock::new(None),
            #[cfg(feature = "inspector")]
            inspector: RwLock::new(None),
            plugin_manager: RwLock::new(None),
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

    pub fn get_plugin_manager(&self) -> Arc<PluginManager> {
        self.plugin_manager
            .read()
            .as_ref()
            .cloned()
            .expect("plugin_manager not initialized")
    }

    pub fn set_plugin_manager(&self, manager: Arc<PluginManager>) {
        *self.plugin_manager.write() = Some(manager);
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
