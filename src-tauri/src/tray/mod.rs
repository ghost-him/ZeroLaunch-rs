use parking_lot::RwLock;
use std::sync::Arc;
use tauri::{
    image::Image,
    menu::{CheckMenuItem, Menu, MenuBuilder, MenuItem},
    tray::{TrayIcon, TrayIconBuilder, TrayIconEvent},
    App, AppHandle, Manager, Runtime,
};
use tracing::{debug, error, info, warn};

use crate::core::app_command;
use crate::sdk::HostApi;

const MENU_ID_SHOW_SETTINGS: &str = "show_setting_window";
const MENU_ID_REFRESH: &str = "refresh_candidates";
const MENU_ID_REREGISTER_HOTKEY: &str = "reregister_hotkey";
const MENU_ID_TOGGLE_GAME_MODE: &str = "toggle_game_mode";
const MENU_ID_EXIT_PROGRAM: &str = "exit_program";

enum MenuEventId {
    ShowSettingWindow,
    RefreshCandidates,
    ReregisterHotkeys,
    ToggleGameMode,
    ExitProgram,
    Unknown(String),
}

impl From<&str> for MenuEventId {
    fn from(id: &str) -> Self {
        match id {
            MENU_ID_SHOW_SETTINGS => MenuEventId::ShowSettingWindow,
            MENU_ID_REFRESH => MenuEventId::RefreshCandidates,
            MENU_ID_REREGISTER_HOTKEY => MenuEventId::ReregisterHotkeys,
            MENU_ID_TOGGLE_GAME_MODE => MenuEventId::ToggleGameMode,
            MENU_ID_EXIT_PROGRAM => MenuEventId::ExitProgram,
            _ => MenuEventId::Unknown(id.to_string()),
        }
    }
}

/// 系统托盘管理器。
///
/// 负责托盘图标、右键菜单和事件处理的全生命周期。
/// 采用 Inner 模式：外层 TrayManager 通过 RwLock 委托所有操作给 Inner。
pub struct TrayManager {
    inner: RwLock<TrayManagerInner>,
}

struct TrayManagerInner {
    tray_icon: Option<TrayIcon>,
    menu: Option<Menu<tauri::Wry>>,
    host_api: Arc<HostApi>,
    app_handle: Option<AppHandle>,
    /// 游戏模式复选框菜单项，用于在事件处理中切换勾选状态
    game_mode_item: Option<CheckMenuItem<tauri::Wry>>,
}

impl TrayManager {
    /// 创建 TrayManager 实例。
    ///
    /// 参数：host_api - 用于解析内置图标路径。
    /// 命令发送通过全局 `app_command::send()` 完成，不再作为参数注入——
    /// 因为命令通道是应用基础设施（有且仅有一个消费者），不应伪装为组件的业务依赖。
    pub fn new(host_api: Arc<HostApi>) -> Self {
        Self {
            inner: RwLock::new(TrayManagerInner {
                tray_icon: None,
                menu: None,
                host_api,
                app_handle: None,
                game_mode_item: None,
            }),
        }
    }

    /// 初始化系统托盘（注册事件、创建图标，含失败重试）。
    ///
    /// 参数：app - Tauri App 实例，用于注册托盘双击事件。
    pub async fn init(&self, app: &mut App) {
        let app_handle = app.handle().clone();

        // 注册双击事件：切换主窗口显示/隐藏
        app.on_tray_icon_event(move |tray_app_handle, event| {
            if let TrayIconEvent::DoubleClick { .. } = event {
                if let Some(window) = tray_app_handle.get_webview_window("main") {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                    } else {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
        });

        // 存储应用句柄
        {
            let mut inner = self.inner.write();
            inner.app_handle = Some(app_handle.clone());
        }

        // 首次尝试创建托盘
        if self.try_create_and_set_tray(&app_handle).is_ok() {
            debug!("System tray initialized successfully on first attempt.");
            return;
        }

        // 重试逻辑
        let retry_delays = [1, 2, 2, 3, 5];
        for &delay in &retry_delays {
            warn!(
                "Tray icon creation failed. Retrying in {} seconds...",
                delay
            );
            tokio::time::sleep(std::time::Duration::from_secs(delay)).await;

            if self.try_create_and_set_tray(&app_handle).is_ok() {
                info!("System tray initialized successfully after retry.");
                return;
            }
        }

        error!("Failed to initialize system tray after all retries.");
    }

    /// 更新游戏模式复选框状态。
    pub fn set_game_mode_checked(&self, checked: bool) {
        if let Some(ref item) = self.inner.read().game_mode_item {
            let _ = item.set_checked(checked);
        }
    }

    /// 根据当前系统主题更新托盘图标。
    pub fn update_icon_theme(&self) {
        let inner = self.inner.read();
        let app_handle = match &inner.app_handle {
            Some(h) => h,
            None => {
                warn!("TrayManager not initialized, skipping icon theme update");
                return;
            }
        };

        let use_white_icon = should_use_white_tray_icon(app_handle);
        let icon_key = if use_white_icon {
            "tray_icon_white"
        } else {
            "tray_icon"
        };

        if let Some(path) = inner.host_api.get_app_icon_path(icon_key) {
            match Image::from_path(&path) {
                Ok(icon) => {
                    if let Some(ref tray_icon) = inner.tray_icon {
                        if let Err(e) = tray_icon.set_icon(Some(icon)) {
                            warn!("Failed to update tray icon: {:?}", e);
                        } else {
                            debug!("Tray icon updated to {}", icon_key);
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to load tray icon from path {:?}: {:?}", path, e);
                }
            }
        } else {
            warn!(
                "Tray icon key '{}' not found in app resource service",
                icon_key
            );
        }
    }

    /// 重建托盘菜单（用于语言切换等场景）。
    pub fn update_menu_language(&self) {
        let inner = self.inner.read();
        let app_handle = match &inner.app_handle {
            Some(h) => h,
            None => {
                warn!("TrayManager not initialized, skipping menu language update");
                return;
            }
        };

        // 保留当前游戏模式勾选状态
        let game_mode_checked = inner
            .game_mode_item
            .as_ref()
            .and_then(|item| item.is_checked().ok())
            .unwrap_or(false);

        let toggle_game_mode = match CheckMenuItem::with_id(
            app_handle,
            MENU_ID_TOGGLE_GAME_MODE,
            "游戏模式",
            true,
            game_mode_checked,
            None::<&str>,
        ) {
            Ok(item) => item,
            Err(e) => {
                warn!("Failed to recreate game mode menu item: {:?}", e);
                return;
            }
        };

        let menu = match build_tray_menu(app_handle, &toggle_game_mode) {
            Ok(m) => m,
            Err(e) => {
                warn!("Failed to rebuild tray menu: {:?}", e);
                return;
            }
        };

        if let Some(ref tray_icon) = inner.tray_icon {
            if let Err(e) = tray_icon.set_menu(Some(menu.clone())) {
                warn!("Failed to update tray menu: {:?}", e);
            }
            if let Err(e) = tray_icon.set_tooltip(Some("zerolaunch-rs")) {
                warn!("Failed to update tray tooltip: {:?}", e);
            }
        }
        drop(inner);

        let mut inner = self.inner.write();
        inner.menu = Some(menu);
        inner.game_mode_item = Some(toggle_game_mode);
        debug!("Tray menu language updated.");
    }

    /// 退出程序。
    pub fn exit_program(&self) {
        let inner = self.inner.read();
        if let Some(ref app_handle) = inner.app_handle {
            app_handle.exit(0);
        }
    }

    // ===== 内部方法 =====

    /// 尝试创建托盘图标并保存到 Inner。
    /// Inner 层方法，由外壳委托调用。
    fn try_create_and_set_tray(&self, app_handle: &AppHandle) -> Result<(), ()> {
        // 先创建游戏模式复选框，持有引用后再构建菜单
        let toggle_game_mode = CheckMenuItem::with_id(
            app_handle,
            MENU_ID_TOGGLE_GAME_MODE,
            "游戏模式",
            true,
            false,
            None::<&str>,
        )
        .map_err(|e| {
            warn!("Failed to create game mode menu item: {:?}", e);
        })?;

        let menu = build_tray_menu(app_handle, &toggle_game_mode).map_err(|e| {
            warn!("Failed to build tray menu: {:?}", e);
        })?;

        let tray_icon = self.create_tray_icon(app_handle, &menu).map_err(|e| {
            warn!("Failed to create tray icon: {:?}", e);
        })?;

        let mut inner = self.inner.write();
        inner.tray_icon = Some(tray_icon);
        inner.menu = Some(menu);
        inner.game_mode_item = Some(toggle_game_mode);

        Ok(())
    }

    /// 创建托盘图标实例。
    /// Inner 层方法，由外壳委托调用。
    fn create_tray_icon(
        &self,
        app_handle: &AppHandle,
        menu: &Menu<tauri::Wry>,
    ) -> tauri::Result<TrayIcon> {
        let use_white_icon = should_use_white_tray_icon(app_handle);
        let icon_key = if use_white_icon {
            "tray_icon_white"
        } else {
            "tray_icon"
        };

        let inner = self.inner.read();
        let icon_path = inner
            .host_api
            .get_app_icon_path(icon_key)
            .unwrap_or_else(|| panic!("Tray icon path '{}' not found in app resources", icon_key));
        drop(inner);

        let icon = Image::from_path(&icon_path)
            .unwrap_or_else(|_| panic!("Failed to load tray icon from path {:?}", icon_path));

        TrayIconBuilder::new()
            .menu(menu)
            .icon(icon)
            .tooltip("zerolaunch-rs")
            .show_menu_on_left_click(false)
            .on_menu_event(move |_app, event| {
                let event_id = MenuEventId::from(event.id().as_ref());
                match event_id {
                    MenuEventId::ShowSettingWindow => {
                        app_command::send(app_command::AppCommand::ShowSettings)
                    }
                    MenuEventId::ExitProgram => {
                        app_command::send(app_command::AppCommand::ExitProgram)
                    }
                    MenuEventId::RefreshCandidates => {
                        app_command::send(app_command::AppCommand::RefreshCandidates);
                    }
                    MenuEventId::ReregisterHotkeys => {
                        app_command::send(app_command::AppCommand::ReregisterHotkeys);
                    }
                    MenuEventId::ToggleGameMode => {
                        // 游戏模式的视觉状态在消费者 task 中通过 set_game_mode_checked 更新
                        app_command::send(app_command::AppCommand::ToggleGameMode);
                    }
                    MenuEventId::Unknown(id) => {
                        warn!("Unknown tray menu event: {}", id);
                    }
                }
                debug!("Menu ID: {}", event.id().0);
            })
            .build(app_handle)
    }
}

// ===== 自由函数（供闭包使用，不捕获 TrayManager） =====

fn build_tray_menu<R: Runtime>(
    app_handle: &AppHandle<R>,
    toggle_game_mode: &CheckMenuItem<R>,
) -> tauri::Result<Menu<R>> {
    let show_settings = MenuItem::with_id(
        app_handle,
        MENU_ID_SHOW_SETTINGS,
        "设置窗口",
        true,
        None::<&str>,
    )?;
    let refresh = MenuItem::with_id(
        app_handle,
        MENU_ID_REFRESH,
        "刷新数据库",
        true,
        None::<&str>,
    )?;
    let reregister = MenuItem::with_id(
        app_handle,
        MENU_ID_REREGISTER_HOTKEY,
        "重新注册快捷键",
        true,
        None::<&str>,
    )?;
    let exit_program = MenuItem::with_id(
        app_handle,
        MENU_ID_EXIT_PROGRAM,
        "退出程序",
        true,
        None::<&str>,
    )?;

    MenuBuilder::new(app_handle)
        .item(&show_settings)
        .separator()
        .item(&refresh)
        .item(&reregister)
        .item(toggle_game_mode)
        .separator()
        .item(&exit_program)
        .build()
}

fn should_use_white_tray_icon(app_handle: &AppHandle) -> bool {
    if let Some(window) = app_handle.get_webview_window("main") {
        match window.theme() {
            Ok(theme) => theme == tauri::Theme::Dark,
            Err(_) => false,
        }
    } else {
        false
    }
}
