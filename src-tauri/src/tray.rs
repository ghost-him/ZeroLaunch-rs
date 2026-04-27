use std::sync::Arc;
use tauri::{
    image::Image,
    menu::{Menu, MenuBuilder, MenuItem},
    tray::{TrayIcon, TrayIconBuilder, TrayIconEvent},
    App, AppHandle, Manager, Runtime,
};
use tracing::{debug, error, info, warn};

use crate::error::{OptionExt, ResultExt};

use crate::{AppState, ServiceLocator};

const MENU_ID_SHOW_SETTINGS: &str = "show_setting_window";
const MENU_ID_EXIT_PROGRAM: &str = "exit_program";

enum MenuEventId {
    ShowSettingWindow,
    ExitProgram,
    Unknown(String),
}

impl From<&str> for MenuEventId {
    fn from(id: &str) -> Self {
        match id {
            MENU_ID_SHOW_SETTINGS => MenuEventId::ShowSettingWindow,
            MENU_ID_EXIT_PROGRAM => MenuEventId::ExitProgram,
            _ => MenuEventId::Unknown(id.to_string()),
        }
    }
}

pub fn handle_show_settings_window() {
    if let Err(e) = show_setting_window() {
        warn!("Failed to show setting window: {:?}", e);
    }
}

pub async fn handle_exit_program(app_handle: &AppHandle) {
    app_handle.exit(0);
}

fn build_tray_menu<R: Runtime>(app_handle: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let show_settings = MenuItem::with_id(
        app_handle,
        MENU_ID_SHOW_SETTINGS,
        "设置窗口",
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

fn create_tray_icon<R: Runtime>(app_handle: &AppHandle, menu: Menu<R>) -> tauri::Result<TrayIcon> {
    let use_white_icon = should_use_white_tray_icon(app_handle);

    let icon_key = if use_white_icon {
        "tray_icon_white"
    } else {
        "tray_icon"
    };

    let state = app_handle.state::<Arc<AppState>>();
    let host_api = state.get_host_api();
    let tray_icon_path_value = host_api
        .get_app_icon_path(icon_key)
        .expect_programming(&format!(
            "Tray icon path '{}' not found in app resource service",
            icon_key
        ));
    let icon = Image::from_path(&tray_icon_path_value).expect_programming(&format!(
        "无法从路径 {:?} 加载托盘图标",
        tray_icon_path_value
    ));

    TrayIconBuilder::new()
        .menu(&menu)
        .icon(icon)
        .tooltip("zerolaunch-rs")
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| {
            let event_id = MenuEventId::from(event.id().as_ref());
            match event_id {
                MenuEventId::ShowSettingWindow => handle_show_settings_window(),
                MenuEventId::ExitProgram => {
                    let app_clone = app.clone();
                    tauri::async_runtime::spawn(async move {
                        handle_exit_program(&app_clone).await;
                    });
                }
                MenuEventId::Unknown(id) => {
                    warn!("Unknown menu event: {}", id);
                }
            }
            debug!("Menu ID: {}", event.id().0);
        })
        .build(app_handle)
}

pub async fn init_system_tray(app: &mut App) {
    let app_handle = app.handle().clone();

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

    if try_create_and_set_tray(&app_handle).is_ok() {
        debug!("System tray initialized successfully on first attempt.");
        return;
    }

    let retry_delays = [1, 2, 2, 3, 5];
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

    let state = app_handle.state::<Arc<AppState>>();
    state.set_tray_icon(Arc::new(tray_icon));
    state.set_tray_menu(Arc::new(menu));

    Ok(())
}

pub fn update_tray_menu_language() {
    let state = ServiceLocator::get_state();
    let app_handle = state.get_main_handle();

    let menu = match build_tray_menu(&app_handle) {
        Ok(m) => m,
        Err(e) => {
            warn!("Failed to rebuild tray menu: {:?}", e);
            return;
        }
    };

    let tray_icon = state.get_tray_icon();

    if let Err(e) = tray_icon.set_menu(Some(menu.clone())) {
        warn!("Failed to update tray menu: {:?}", e);
    }

    state.set_tray_menu(Arc::new(menu));

    let tooltip = "zerolaunch-rs";
    if let Err(e) = tray_icon.set_tooltip(Some(tooltip)) {
        warn!("Failed to update tray tooltip: {:?}", e);
    }

    debug!("Tray menu language updated.");
}

pub fn update_tray_icon_theme() {
    let state = ServiceLocator::get_state();
    let app_handle = state.get_main_handle();

    let use_white_icon = should_use_white_tray_icon(&app_handle);

    let icon_key = if use_white_icon {
        "tray_icon_white"
    } else {
        "tray_icon"
    };

    let host_api = state.get_host_api();
    if let Some(path) = host_api.get_app_icon_path(icon_key) {
        match Image::from_path(&path) {
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
        warn!(
            "Tray icon key '{}' not found in app resource service",
            icon_key
        );
    }
}

fn show_setting_window() -> Result<(), Box<dyn std::error::Error>> {
    let state = ServiceLocator::get_state();
    let app_handle = state.get_main_handle();
    if let Some(window) = app_handle.get_webview_window("setting_window") {
        window.show()?;
        window.set_focus()?;
    }
    Ok(())
}
