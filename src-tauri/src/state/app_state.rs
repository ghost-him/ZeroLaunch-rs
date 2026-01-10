use crate::core::storage::storage_manager::StorageManager;
use crate::error::OptionExt;
use crate::modules::bookmark_loader::BookmarkLoader;
#[cfg(target_arch = "x86_64")]
use crate::modules::everything::EverythingManager;
use crate::modules::icon_manager::IconManager;
use crate::modules::refresh_scheduler::RefreshScheduler;
use crate::modules::shortcut_manager::ShortcutManager;
use crate::modules::{config::config_manager::RuntimeConfig, program_manager::ProgramManager};
use crate::utils::i18n::Translator;
use crate::utils::waiting_hashmap::AsyncWaitingHashMap;
use parking_lot::RwLock;
use std::sync::Arc;
use tauri::menu::Menu;
use tauri::tray::TrayIcon;
use tauri::AppHandle;

pub struct AppState {
    /// 运行时配置
    runtime_config: RwLock<Option<Arc<RuntimeConfig>>>,
    /// 程序管理器
    program_manager: RwLock<Option<Arc<ProgramManager>>>,
    /// 主窗口句柄
    main_handle: RwLock<Option<Arc<AppHandle>>>,
    /// 刷新调度器
    refresh_scheduler: RwLock<Option<Arc<RefreshScheduler>>>,
    /// 当前的窗口是否可见
    is_search_bar_visible: RwLock<bool>,
    /// 文件存储器
    storage_client: RwLock<Option<Arc<StorageManager>>>,
    /// 消息队列(目前没用，本来用于onedrive的验证码传递)
    waiting_hashmap: Arc<AsyncWaitingHashMap<String, Vec<(String, String)>>>,
    /// 系统托盘
    tray_icon: RwLock<Option<Arc<TrayIcon>>>,
    /// 托盘菜单
    tray_menu: RwLock<Option<Arc<Menu<tauri::Wry>>>>,
    /// 快捷键管理器
    shortcut_manager: RwLock<Option<Arc<ShortcutManager>>>,
    /// 游戏模式
    game_mode: RwLock<bool>,
    /// 阻止所有的键盘输入
    is_keyboard_blocked: RwLock<bool>,
    /// 国际化翻译器
    translator: Arc<RwLock<Translator>>,
    /// 唤醒搜索栏前的前台窗口句柄
    previous_foreground_window: RwLock<Option<isize>>,
    /// 唤醒搜索栏前活动窗口的选中文本
    previous_selection: RwLock<Option<String>>,
    /// Everything 管理器
    #[cfg(target_arch = "x86_64")]
    everything_manager: Arc<EverythingManager>,
    /// 图标管理器
    icon_manager: RwLock<Option<Arc<IconManager>>>,
    /// 书签加载器
    bookmark_loader: RwLock<Option<Arc<BookmarkLoader>>>,
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
            refresh_scheduler: RwLock::new(None),
            is_search_bar_visible: RwLock::new(false),
            storage_client: RwLock::new(None),
            waiting_hashmap: Arc::new(AsyncWaitingHashMap::new()),
            tray_icon: RwLock::new(None),
            tray_menu: RwLock::new(None),
            shortcut_manager: RwLock::new(None),
            game_mode: RwLock::new(false),
            is_keyboard_blocked: RwLock::new(false),
            translator: Arc::new(RwLock::new(Translator::new())),
            previous_foreground_window: RwLock::new(None),
            previous_selection: RwLock::new(None),
            #[cfg(target_arch = "x86_64")]
            everything_manager: Arc::new(EverythingManager::new()),
            icon_manager: RwLock::new(None),
            bookmark_loader: RwLock::new(None),
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

    // region: RefreshScheduler 访问方法
    /// 获取刷新调度器的克隆
    pub fn get_refresh_scheduler(&self) -> Arc<RefreshScheduler> {
        self.refresh_scheduler
            .read()
            .as_ref()
            .cloned()
            .expect_programming("refresh scheduler not initialized")
    }

    /// 设置刷新调度器
    pub fn set_refresh_scheduler(&self, scheduler: Arc<RefreshScheduler>) {
        *self.refresh_scheduler.write() = Some(scheduler);
    }
    // endregion

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

    /// 获取翻译器的 RwLock 引用
    pub fn get_translator(&self) -> Arc<RwLock<Translator>> {
        Arc::clone(&self.translator)
    }

    /// 设置唤醒前的前台窗口句柄
    pub fn set_previous_foreground_window(&self, hwnd: Option<isize>) {
        *self.previous_foreground_window.write() = hwnd;
    }

    /// 获取唤醒前的前台窗口句柄
    pub fn get_previous_foreground_window(&self) -> Option<isize> {
        *self.previous_foreground_window.read()
    }

    /// 设置唤醒前活动窗口的选中文本
    pub fn set_previous_selection(&self, selection: Option<String>) {
        *self.previous_selection.write() = selection;
    }

    /// 获取唤醒前活动窗口的选中文本
    pub fn get_previous_selection(&self) -> Option<String> {
        self.previous_selection.read().clone()
    }

    /// 获取 Everything 管理器的克隆
    #[cfg(target_arch = "x86_64")]
    pub fn get_everything_manager(&self) -> Arc<EverythingManager> {
        self.everything_manager.clone()
    }

    /// 获取图标管理器的克隆
    pub fn get_icon_manager(&self) -> Arc<IconManager> {
        self.icon_manager
            .read()
            .as_ref()
            .cloned()
            .expect_programming("icon manager not initialized")
    }

    /// 设置图标管理器
    pub fn set_icon_manager(&self, icon_manager: Arc<IconManager>) {
        *self.icon_manager.write() = Some(icon_manager);
    }

    /// 获取书签加载器的克隆
    pub fn get_bookmark_loader(&self) -> Arc<BookmarkLoader> {
        self.bookmark_loader
            .read()
            .as_ref()
            .cloned()
            .expect_programming("bookmark loader not initialized")
    }

    /// 设置书签加载器
    pub fn set_bookmark_loader(&self, bookmark_loader: Arc<BookmarkLoader>) {
        *self.bookmark_loader.write() = Some(bookmark_loader);
    }
}

// Custom Debug implementation for AppState
impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("runtime_config", &self.runtime_config)
            .field("program_manager", &self.program_manager)
            .field("main_handle", &self.main_handle)
            .field("refresh_scheduler", &"<RefreshScheduler>")
            .field("storage_client", &self.storage_client)
            .field("waiting_hashmap", &self.waiting_hashmap)
            .finish()
    }
}
