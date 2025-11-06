use crate::logging::log_application_shutdown;
use std::sync::Arc;
use tauri::{
    image::Image,
    menu::{CheckMenuItem, Menu, MenuBuilder, MenuItem},
    tray::{TrayIcon, TrayIconBuilder, TrayIconEvent},
    App, AppHandle, Manager, Runtime,
};
use tracing::{debug, warn};

use crate::error::{OptionExt, ResultExt};
use crate::utils::i18n::{t, t_with};
use crate::utils::notify::notify;
use crate::{
    handle_pressed, save_config_to_file, show_setting_window, update_app_setting, AppState,
    ServiceLocator, APP_PIC_PATH,
};
// Removed: use crate::retry_register_shortcut; // Appears unused, functionality merged
use crate::modules::config::default::APP_VERSION;

// --- Constants for Menu Event IDs ---
const MENU_ID_SHOW_SETTINGS: &str = "show_setting_window";
const MENU_ID_EXIT_PROGRAM: &str = "exit_program";
const MENU_ID_UPDATE_APP_SETTING: &str = "update_app_setting";
const MENU_ID_RETRY_REGISTER_SHORTCUT: &str = "retry_register_shortcut";
const MENU_ID_SWITCH_GAME_MODE: &str = "switch_game_mode";

// --- Enum for Menu Event IDs ---
enum MenuEventId {
    ShowSettingWindow,
    ExitProgram,
    UpdateAppSetting,
    RegisterShortcut,
    SwitchGameMode,
    Unknown(String),
}

impl From<&str> for MenuEventId {
    fn from(id: &str) -> Self {
        match id {
            MENU_ID_SHOW_SETTINGS => MenuEventId::ShowSettingWindow,
            MENU_ID_EXIT_PROGRAM => MenuEventId::ExitProgram,
            MENU_ID_UPDATE_APP_SETTING => MenuEventId::UpdateAppSetting,
            MENU_ID_RETRY_REGISTER_SHORTCUT => MenuEventId::RegisterShortcut,
            MENU_ID_SWITCH_GAME_MODE => MenuEventId::SwitchGameMode,
            _ => MenuEventId::Unknown(id.to_string()),
        }
    }
}

// 原先用于菜单图标的辅助函数已移除；托盘菜单现在不使用图标。

// --- Menu Item Handlers ---

pub fn handle_show_settings_window() {
    if let Err(e) = show_setting_window() {
        warn!("Failed to show setting window: {:?}", e);
    }
}

pub async fn handle_exit_program(app_handle: &AppHandle) {
    save_config_to_file(false).await;
    ServiceLocator::get_state()
        .get_storage_manager()
        .upload_all_file_force()
        .await;

    // 记录应用关闭信息
    log_application_shutdown();

    app_handle.exit(0);
}

pub async fn handle_update_app_setting() {
    update_app_setting().await;
}

pub fn handle_register_shortcut() {
    let state = ServiceLocator::get_state();
    if state.get_game_mode() {
        notify("ZeroLaunch-rs", &t("notifications.close_game_mode_first"));
        return;
    }
    let shortcut_manager = state.get_shortcut_manager();
    if let Err(e) = shortcut_manager.register_all_shortcuts() {
        warn!("Failed to register all shortcuts: {:?}", e);
        notify(
            "ZeroLaunch-rs",
            &t("notifications.shortcut_register_failed"),
        );
    } else {
        notify("ZeroLaunch-rs", &t("notifications.shortcut_registered"));
    }
}

/// 切换游戏模式（禁用/启用全局快捷键）
///
/// 此函数会切换游戏模式状态，并同步更新托盘菜单中的复选框状态
pub fn handle_toggle_game_mode() {
    let state = ServiceLocator::get_state();
    let shortcut_manager = state.get_shortcut_manager();

    // 切换游戏模式状态
    let new_game_mode = !state.get_game_mode();
    state.set_game_mode(new_game_mode);

    // 根据新的游戏模式状态，注册或注销快捷键
    if new_game_mode {
        if let Err(e) = shortcut_manager.unregister_all_shortcut() {
            warn!("Failed to unregister shortcuts for game mode: {:?}", e);
        }
        notify("ZeroLaunch-rs", &t("notifications.game_mode_enabled"));
    } else {
        if let Err(e) = shortcut_manager.register_all_shortcuts() {
            warn!(
                "Failed to register shortcuts after exiting game mode: {:?}",
                e
            );
        }
        notify("ZeroLaunch-rs", &t("notifications.game_mode_disabled"));
    }

    // 同步更新托盘菜单中的复选框状态
    update_game_mode_menu_state(new_game_mode);
}

/// 更新托盘菜单中游戏模式复选框的状态
fn update_game_mode_menu_state(checked: bool) {
    let state = ServiceLocator::get_state();
    let tray_menu = state.get_tray_menu();

    if let Some(item) = tray_menu.get(MENU_ID_SWITCH_GAME_MODE) {
        if let Some(menu_item) = item.as_check_menuitem() {
            if let Err(e) = menu_item.set_checked(checked) {
                warn!(
                    "Failed to update game mode menu item checked state: {:?}",
                    e
                );
            }
        } else {
            warn!("Game mode menu item is not a CheckMenuItem.");
        }
    } else {
        warn!("Could not find game mode menu item.");
    }
}

// --- Tray Menu and Icon Creation ---

/// 使用 MenuItem::with_id 与 CheckMenuItem::with_id 构建系统托盘菜单（无图标）。
fn build_tray_menu<R: Runtime>(app_handle: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    // 读取当前游戏模式状态以初始化复选项
    let game_mode = ServiceLocator::get_state().get_game_mode();

    let show_settings = MenuItem::with_id(
        app_handle,
        MENU_ID_SHOW_SETTINGS,
        t("tray.show_settings"),
        true,
        None::<&str>,
    )?;
    let update_app_setting = MenuItem::with_id(
        app_handle,
        MENU_ID_UPDATE_APP_SETTING,
        t("tray.refresh_database"),
        true,
        None::<&str>,
    )?;
    let retry_shortcut = MenuItem::with_id(
        app_handle,
        MENU_ID_RETRY_REGISTER_SHORTCUT,
        t("tray.retry_register_shortcut"),
        true,
        None::<&str>,
    )?;
    let game_mode_item = CheckMenuItem::with_id(
        app_handle,
        MENU_ID_SWITCH_GAME_MODE,
        t("tray.switch_game_mode"),
        true,
        game_mode,
        None::<&str>,
    )?;
    let exit_program = MenuItem::with_id(
        app_handle,
        MENU_ID_EXIT_PROGRAM,
        t("tray.exit_program"),
        true,
        None::<&str>,
    )?;

    MenuBuilder::new(app_handle)
        .item(&show_settings)
        .separator()
        .item(&update_app_setting)
        .item(&retry_shortcut)
        .item(&game_mode_item)
        .separator()
        .item(&exit_program)
        .build()
}

/// Creates and configures the system tray icon.
fn create_tray_icon<R: Runtime>(app_handle: &AppHandle, menu: Menu<R>) -> tauri::Result<TrayIcon> {
    let tray_icon_path_value = APP_PIC_PATH
        .get("tray_icon")
        .expect_programming("Tray icon path 'tray_icon' not found in APP_PIC_PATH")
        .clone();
    let icon = Image::from_path(&tray_icon_path_value).expect_programming(&format!(
        "无法从路径 {:?} 加载托盘图标",
        tray_icon_path_value
    ));

    TrayIconBuilder::new()
        .menu(&menu)
        .icon(icon)
        .tooltip(t_with("tray.tooltip", &[("version", &APP_VERSION.clone())]))
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| {
            let event_id = MenuEventId::from(event.id().as_ref());
            // It's often better to pass the specific menu item if needed,
            // rather than the whole menu, but for `switch_game_mode` we need it.
            match event_id {
                MenuEventId::ShowSettingWindow => handle_show_settings_window(),
                MenuEventId::ExitProgram => {
                    let app_clone = app.clone();
                    tauri::async_runtime::spawn(async move {
                        // Spawn to avoid blocking, then block_on inside if necessary
                        handle_exit_program(&app_clone).await;
                    });
                }
                MenuEventId::UpdateAppSetting => {
                    tauri::async_runtime::spawn(handle_update_app_setting());
                }
                MenuEventId::RegisterShortcut => handle_register_shortcut(),
                MenuEventId::SwitchGameMode => handle_toggle_game_mode(),
                MenuEventId::Unknown(id) => {
                    warn!("Unknown menu event: {}", id);
                }
            }
            debug!("Menu ID: {}", event.id().0);
        })
        .build(app_handle)
}

// --- Main Initialization Function ---

/// Initializes the system tray icon and menu for the application.
pub fn init_system_tray(app: &mut App) {
    let app_handle = app.handle().clone();

    let menu = match build_tray_menu(&app_handle) {
        Ok(m) => m,
        Err(e) => {
            warn!("Failed to build tray menu: {:?}", e);
            // Optionally, create a minimal fallback menu or panic
            return;
        }
    };

    let tray_icon = match create_tray_icon(&app_handle, menu.clone()) {
        Ok(icon) => icon,
        Err(e) => {
            warn!("Failed to create tray icon: {:?}", e);
            return;
        }
    };

    // Store the tray icon and menu in app state
    let state = app.state::<Arc<AppState>>();
    state.set_tray_icon(Arc::new(tray_icon));
    state.set_tray_menu(Arc::new(menu));

    // Handle other tray icon events (e.g., double click)
    app.on_tray_icon_event(move |tray_app_handle, event| {
        if let TrayIconEvent::DoubleClick { .. } = event {
            handle_pressed(tray_app_handle);
        }
    });

    debug!("System tray initialized.");
}

/// 更新托盘菜单的语言
///
/// 当用户切换应用语言时调用，重新构建托盘菜单以显示新语言的文本
pub fn update_tray_menu_language() {
    let state = ServiceLocator::get_state();
    let app_handle = state.get_main_handle();

    // 重新构建菜单
    let menu = match build_tray_menu(&app_handle) {
        Ok(m) => m,
        Err(e) => {
            warn!("Failed to rebuild tray menu: {:?}", e);
            return;
        }
    };

    // 更新托盘图标的菜单和tooltip
    let tray_icon = state.get_tray_icon();

    if let Err(e) = tray_icon.set_menu(Some(menu.clone())) {
        warn!("Failed to update tray menu: {:?}", e);
    }

    // 更新存储的菜单引用
    state.set_tray_menu(Arc::new(menu));

    // 更新 tooltip
    let tooltip = t_with("tray.tooltip", &[("version", &APP_VERSION.clone())]);
    if let Err(e) = tray_icon.set_tooltip(Some(tooltip)) {
        warn!("Failed to update tray tooltip: {:?}", e);
    }

    debug!("Tray menu language updated.");
}
