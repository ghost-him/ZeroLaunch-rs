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
use crate::{
    handle_pressed, notify, save_config_to_file, show_setting_window, update_app_setting, AppState,
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

fn handle_show_settings_window() {
    if let Err(e) = show_setting_window() {
        warn!("Failed to show setting window: {:?}", e);
    }
}

async fn handle_exit_program(app_handle: &AppHandle) {
    save_config_to_file(false).await;
    ServiceLocator::get_state()
        .get_storage_manager()
        .upload_all_file_force()
        .await;

    // 记录应用关闭信息
    log_application_shutdown();

    app_handle.exit(0);
}

async fn handle_update_app_setting() {
    update_app_setting().await;
}

fn handle_register_shortcut() {
    let state = ServiceLocator::get_state();
    if state.get_game_mode() {
        notify("ZeroLaunch-rs", "请先关闭游戏模式后再尝试重新注册快捷键。");
        return;
    }
    let shortcut_manager = state.get_shortcut_manager();
    if let Err(e) = shortcut_manager.register_all_shortcuts() {
        warn!("Failed to register all shortcuts: {:?}", e);
        notify("ZeroLaunch-rs", "快捷键注册失败，请查看日志。");
    } else {
        notify("ZeroLaunch-rs", "快捷键已重新注册。");
    }
}

fn handle_switch_game_mode<R: Runtime>(game_mode_item: &CheckMenuItem<R>) {
    let state = ServiceLocator::get_state();
    let shortcut_manager = state.get_shortcut_manager();

    let target_game_mode = !state.get_game_mode();
    state.set_game_mode(target_game_mode);

    if target_game_mode {
        if let Err(e) = shortcut_manager.unregister_all_shortcut() {
            warn!("Failed to unregister shortcuts for game mode: {:?}", e);
        }
        if let Err(e) = game_mode_item.set_text("关闭游戏模式") {
            warn!("Failed to set menu item text for game mode (on): {:?}", e);
        }
        notify("ZeroLaunch-rs", "游戏模式已开启，全局快捷键已禁用。");
    } else {
        if let Err(e) = shortcut_manager.register_all_shortcuts() {
            warn!(
                "Failed to register shortcuts after exiting game mode: {:?}",
                e
            );
        }
        if let Err(e) = game_mode_item.set_text("开启游戏模式") {
            warn!("Failed to set menu item text for game mode (off): {:?}", e);
        }
        notify("ZeroLaunch-rs", "游戏模式已关闭，全局快捷键已启用。");
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
        "打开设置界面",
        true,
        None::<&str>,
    )?;
    let update_app_setting = MenuItem::with_id(
        app_handle,
        MENU_ID_UPDATE_APP_SETTING,
        "刷新数据库",
        true,
        None::<&str>,
    )?;
    let retry_shortcut = MenuItem::with_id(
        app_handle,
        MENU_ID_RETRY_REGISTER_SHORTCUT,
        "重新注册快捷键",
        true,
        None::<&str>,
    )?;
    let game_mode_item = CheckMenuItem::with_id(
        app_handle,
        MENU_ID_SWITCH_GAME_MODE,
        "游戏模式 (禁用全局快捷键)",
        true,
        game_mode,
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
        .tooltip(format!("ZeroLaunch-rs v{}", APP_VERSION.clone()))
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
                MenuEventId::SwitchGameMode => {
                    if let Some(item) = menu.get(MENU_ID_SWITCH_GAME_MODE) {
                        if let Some(menu_item) = item.as_check_menuitem() {
                            handle_switch_game_mode(menu_item);
                        } else {
                            warn!("'Switch Game Mode' menu item is not a CheckMenuItem.");
                        }
                    } else {
                        warn!("Could not find 'Switch Game Mode' menu item by ID.");
                    }
                }
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

    let tray_icon = match create_tray_icon(&app_handle, menu) {
        Ok(icon) => icon,
        Err(e) => {
            warn!("Failed to create tray icon: {:?}", e);
            return;
        }
    };

    // Store the tray icon in app state
    let state = app.state::<Arc<AppState>>();
    state.set_tray_icon(Arc::new(tray_icon)); // tray_icon is already the TrayIcon type
                                              // Handle other tray icon events (e.g., double click)
    app.on_tray_icon_event(move |tray_app_handle, event| {
        if let TrayIconEvent::DoubleClick { .. } = event {
            handle_pressed(tray_app_handle);
        }
    });

    debug!("System tray initialized.");
}
