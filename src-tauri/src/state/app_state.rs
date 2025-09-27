#[cfg(feature = "ai")]
use crate::core::ai::model_manager::ModelManager;
use crate::core::storage::storage_manager::StorageManager;
use crate::error::OptionExt;
use crate::modules::shortcut_manager::ShortcutManager;
use crate::modules::{config::config_manager::RuntimeConfig, program_manager::ProgramManager};
use crate::utils::waiting_hashmap::AsyncWaitingHashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use tauri::tray::TrayIcon;
use tauri::AppHandle;
use timer::{Guard, Timer};

pub struct AppState {
    /// 运行时配置
    runtime_config: RwLock<Option<Arc<RuntimeConfig>>>,
    /// 程序管理器
    program_manager: RwLock<Option<Arc<ProgramManager>>>,
    /// 主窗口句柄
    main_handle: RwLock<Option<Arc<AppHandle>>>,
    /// 定时器守卫
    timer_guard: RwLock<Option<Guard>>,
    /// 定时器
    timer: Arc<Timer>,
    /// 当前的窗口是否可见
    is_search_bar_visible: RwLock<bool>,
    /// 文件存储器
    storage_client: RwLock<Option<Arc<StorageManager>>>,
    /// 消息队列(目前没用，本来用于onedrive的验证码传递)
    waiting_hashmap: Arc<AsyncWaitingHashMap<String, Vec<(String, String)>>>,
    /// 系统托盘
    tray_icon: RwLock<Option<Arc<TrayIcon>>>,
    /// 快捷键管理器
    shortcut_manager: RwLock<Option<Arc<ShortcutManager>>>,
    /// 游戏模式
    game_mode: RwLock<bool>,
    /// 阻止所有的键盘输入
    is_keyboard_blocked: RwLock<bool>,
    /// 模型管理器
    #[cfg(feature = "ai")]
    model_manager: RwLock<Option<Arc<ModelManager>>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            runtime_config: RwLock::new(None),
            program_manager: RwLock::new(None),
            main_handle: RwLock::new(None),
            timer_guard: RwLock::new(None),
            timer: Arc::new(Timer::new()),
            is_search_bar_visible: RwLock::new(false),
            storage_client: RwLock::new(None),
            waiting_hashmap: Arc::new(AsyncWaitingHashMap::new()),
            tray_icon: RwLock::new(None),
            shortcut_manager: RwLock::new(None),
            game_mode: RwLock::new(false),
            is_keyboard_blocked: RwLock::new(false),
            #[cfg(feature = "ai")]
            model_manager: RwLock::new(None),
        }
    }

    // region: Runtime Config 访问方法
    /// 获取运行时配置的克隆
    pub fn get_runtime_config(&self) -> Arc<RuntimeConfig> {
        self.runtime_config
            .read()
            .as_ref()
            .cloned()
            .expect_programming("runtime config not initialized")
    }

    /// 更新运行时配置
    pub fn set_runtime_config(&self, config: Arc<RuntimeConfig>) {
        *self.runtime_config.write() = Some(config);
    }
    // endregion

    // region: Program Manager 访问方法
    /// 获取程序管理器的克隆
    pub fn get_program_manager(&self) -> Arc<ProgramManager> {
        self.program_manager
            .read()
            .as_ref()
            .cloned()
            .expect_programming("program manager not initialized")
    }

    /// 更新程序管理器
    pub fn set_program_manager(&self, manager: Arc<ProgramManager>) {
        *self.program_manager.write() = Some(manager);
    }
    // endregion

    // region: Main Window Handle 访问方法
    /// 获取主窗口句柄的克隆
    pub fn get_main_handle(&self) -> Arc<AppHandle> {
        self.main_handle
            .read()
            .as_ref()
            .cloned()
            .expect_programming("main handle not initialized")
    }

    /// 更新主窗口句柄
    pub fn set_main_handle(&self, handle: Arc<AppHandle>) {
        *self.main_handle.write() = Some(handle);
    }
    // endregion

    pub fn get_timer(&self) -> Arc<Timer> {
        self.timer.clone()
    }

    pub fn set_timer_guard(&self, guard: Guard) {
        *self.timer_guard.write() = Some(guard);
    }

    pub fn take_timer_guard(&self) -> Option<Guard> {
        self.timer_guard.write().take()
    }

    pub fn set_search_bar_visible(&self, is_visible: bool) {
        *self.is_search_bar_visible.write() = is_visible;
    }

    pub fn get_search_bar_visible(&self) -> bool {
        *self.is_search_bar_visible.read()
    }

    /// 获取存储管理器的克隆
    pub fn get_storage_manager(&self) -> Arc<StorageManager> {
        self.storage_client
            .read()
            .as_ref()
            .cloned()
            .expect_programming("storage client not initialized")
    }

    /// 更新存储管理器
    pub fn set_storage_manager(&self, client: Arc<StorageManager>) {
        *self.storage_client.write() = Some(client);
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

    pub fn get_shortcut_manager(&self) -> Arc<ShortcutManager> {
        self.shortcut_manager
            .read()
            .as_ref()
            .cloned()
            .expect_programming("shortcut manager not initialized")
    }

    pub fn set_shortcut_manager(&self, manager: Arc<ShortcutManager>) {
        *self.shortcut_manager.write() = Some(manager);
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

    // region: Model Manager 访问方法
    /// 获取模型管理器的克隆
    #[cfg(feature = "ai")]
    pub fn get_model_manager(&self) -> Arc<ModelManager> {
        self.model_manager.read().as_ref().cloned().expect_programming("model manager not initialized")
    }

    /// 更新模型管理器
    #[cfg(feature = "ai")]
    pub fn set_model_manager(&self, manager: Arc<ModelManager>) {
        *self.model_manager.write() = Some(manager);
    }
    // endregion
}

// Custom Debug implementation for AppState
impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("runtime_config", &self.runtime_config)
            .field("program_manager", &self.program_manager)
            .field("main_handle", &self.main_handle)
            .field("timer_guard", &"<Timer Guard>")
            .field("timer", &"<Timer>")
            .field("storage_client", &self.storage_client)
            .field("waiting_hashmap", &self.waiting_hashmap)
            .finish()
    }
}
