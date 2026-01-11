use std::sync::Arc;
use tauri::{
    image::Image,
    menu::{CheckMenuItem, Menu, MenuBuilder, MenuItem},
    tray::{TrayIcon, TrayIconBuilder, TrayIconEvent},
    App, AppHandle, Manager, Runtime,
};
use tracing::{debug, error, info, warn};

use crate::error::{OptionExt, ResultExt};
use crate::utils::i18n::{t, t_with};
use crate::utils::notify::notify;
use crate::{handle_pressed, show_setting_window, AppState, ServiceLocator, APP_PIC_PATH};
// Removed: use crate::retry_register_shortcut; // Appears unused, functionality merged
use crate::modules::config::default::APP_VERSION;
use crate::modules::config::ui_config::ThemeMode;

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
    app_handle.exit(0);
}

pub fn handle_update_app_setting() {
    let state = ServiceLocator::get_state();
    state.get_refresh_scheduler().trigger_refresh();
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

fn should_use_white_tray_icon(app_handle: &AppHandle) -> bool {
    let state = ServiceLocator::get_state();
    let ui_config = state.get_runtime_config().get_ui_config();
    let tray_theme_mode = ui_config.get_tray_theme_mode();

    match tray_theme_mode {
        ThemeMode::Dark => true,
        ThemeMode::Light => false,
        ThemeMode::System => {
            if let Some(window) = app_handle.get_webview_window("main") {
                match window.theme() {
                    Ok(theme) => theme == tauri::Theme::Dark,
                    Err(_) => false,
                }
            } else {
                false
            }
        }
    }
}

/// Creates and configures the system tray icon.
fn create_tray_icon<R: Runtime>(app_handle: &AppHandle, menu: Menu<R>) -> tauri::Result<TrayIcon> {
    let use_white_icon = should_use_white_tray_icon(app_handle);

    let icon_key = if use_white_icon {
        "tray_icon_white"
    } else {
        "tray_icon"
    };

    let tray_icon_path_value = APP_PIC_PATH
        .get(icon_key)
        .expect_programming(&format!(
            "Tray icon path '{}' not found in APP_PIC_PATH",
            icon_key
        ))
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
                    handle_update_app_setting();
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
pub async fn init_system_tray(app: &mut App) {
    let app_handle = app.handle().clone();

    // Handle other tray icon events (e.g., double click)
    app.on_tray_icon_event(move |tray_app_handle, event| {
        if let TrayIconEvent::DoubleClick { .. } = event {
            handle_pressed(tray_app_handle);
        }
    });

    // Initial attempt
    if try_create_and_set_tray(&app_handle).is_ok() {
        debug!("System tray initialized successfully on first attempt.");
        return;
    }

    // Retry logic
    let retry_delays = [5, 10, 20];
    for &delay in &retry_delays {
        warn!(
            "Tray icon creation failed. Retrying in {} seconds...",
            delay
        );
        tokio::time::sleep(std::time::Duration::from_secs(delay)).await;

        if try_create_and_set_tray(&app_handle).is_ok() {
            info!("System tray initialized successfully after retry.");
            return;
        }
    }

    error!("Failed to initialize system tray after all retries.");
}

fn try_create_and_set_tray(app_handle: &AppHandle) -> Result<(), ()> {
    let menu = match build_tray_menu(app_handle) {
        Ok(m) => m,
        Err(e) => {
            warn!("Failed to build tray menu: {:?}", e);
            return Err(());
        }
    };

    let tray_icon = match create_tray_icon(app_handle, menu.clone()) {
        Ok(icon) => icon,
        Err(e) => {
            warn!("Failed to create tray icon: {:?}", e);
            return Err(());
        }
    };

    // Store the tray icon and menu in app state
    let state = app_handle.state::<Arc<AppState>>();
    state.set_tray_icon(Arc::new(tray_icon));
    state.set_tray_menu(Arc::new(menu));

    Ok(())
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

/// Updates the tray icon based on the current configuration (tray_theme_mode).
pub fn update_tray_icon_theme() {
    let state = ServiceLocator::get_state();
    let app_handle = state.get_main_handle();

    // Determine if we should use the dark mode icon (white icon) or light mode icon (dark icon)
    let use_white_icon = should_use_white_tray_icon(&app_handle);

    let icon_key = if use_white_icon {
        "tray_icon_white"
    } else {
        "tray_icon"
    };

    if let Some(path_entry) = APP_PIC_PATH.get(icon_key) {
        let path = path_entry.value();
        match Image::from_path(path) {
            Ok(icon) => {
                let tray_icon = state.get_tray_icon();
                if let Err(e) = tray_icon.set_icon(Some(icon)) {
                    warn!("Failed to update tray icon: {:?}", e);
                } else {
                    debug!("Tray icon updated to {}", icon_key);
                }
            }
            Err(e) => {
                warn!("Failed to load tray icon from path {:?}: {:?}", path, e);
            }
        }
    } else {
        warn!("Tray icon key '{}' not found in APP_PIC_PATH", icon_key);
    }
}
