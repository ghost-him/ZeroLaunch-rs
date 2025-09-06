use crate::logging::log_application_shutdown;
use std::sync::Arc;
use tauri::{
    image::Image,
    menu::{IconMenuItem, Menu, MenuBuilder}, // Added MenuItem for direct access
    tray::{TrayIcon, TrayIconBuilder, TrayIconEvent}, // Added TrayIcon for type hint
    App,
    AppHandle,
    Manager,
    Runtime, // Added AppHandle
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

// --- Helper function to load icons ---
fn load_icon_or_panic(name: &str) -> Image<'_> {
    let path = APP_PIC_PATH
        .get(name)
        .expect_programming(&format!("图标路径 '{}' 在 APP_PIC_PATH 中未找到", name))
        .clone();
    Image::from_path(&path)
        .expect_programming(&format!("无法从路径 {:?} 加载图标 '{}'", path, name))
}

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

fn handle_switch_game_mode<R: Runtime>(game_mode_item: &IconMenuItem<R>) {
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

/// Builds the system tray menu.
fn build_tray_menu<R: Runtime>(app_handle: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    MenuBuilder::new(app_handle)
        .icon(
            MENU_ID_SHOW_SETTINGS,
            "打开设置界面",
            load_icon_or_panic("settings"),
        )
        .icon(
            MENU_ID_UPDATE_APP_SETTING,
            "刷新数据库",
            load_icon_or_panic("refresh"),
        )
        .icon(
            MENU_ID_RETRY_REGISTER_SHORTCUT,
            "重新注册快捷键",
            load_icon_or_panic("register"),
        )
        .icon(
            MENU_ID_SWITCH_GAME_MODE,
            "开启游戏模式", // Initial text, will be updated
            load_icon_or_panic("game"),
        )
        .icon(MENU_ID_EXIT_PROGRAM, "退出程序", load_icon_or_panic("exit"))
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
                        if let Some(menu_item) = item.as_icon_menuitem() {
                            handle_switch_game_mode(menu_item);
                        } else {
                            warn!("'Switch Game Mode' menu item is not a standard MenuItem.");
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
