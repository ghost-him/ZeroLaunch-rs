use crate::handle_pressed;
use crate::modules::config::default::APP_VERSION;
//use crate::retry_register_shortcut;
use crate::notify;
use crate::save_config_to_file;
use crate::show_setting_window;
use crate::update_app_setting;
use crate::AppState;
use crate::ServiceLocator;
use std::sync::Arc;
use tauri::image::Image;
use tauri::Manager;
use tracing::debug;

use crate::APP_PIC_PATH;
use tauri::menu::MenuBuilder;
use tauri::tray::TrayIconBuilder;
use tauri::tray::TrayIconEvent;
use tauri::App;
use tracing::warn;
enum MenuEventId {
    ShowSettingWindow,
    ExitProgram,
    UpdateAppSetting,
    RegisterShortcut,
    SwitchGameMode,
    Unknown(String),
}

// 从事件 ID 转换为枚举
impl From<&str> for MenuEventId {
    fn from(id: &str) -> Self {
        match id {
            "show_setting_window" => MenuEventId::ShowSettingWindow,
            "exit_program" => MenuEventId::ExitProgram,
            "update_app_setting" => MenuEventId::UpdateAppSetting,
            "retry_register_shortcut" => MenuEventId::RegisterShortcut,
            "switch_game_mode" => MenuEventId::SwitchGameMode,
            _ => MenuEventId::Unknown(id.to_string()),
        }
    }
}

/// 创建一个右键菜单
pub fn init_system_tray(app: &mut App) {
    let handle = app.handle();
    let menu = MenuBuilder::new(app)
        .icon(
            "show_setting_window",
            "打开设置界面",
            Image::from_path(APP_PIC_PATH.get("settings").unwrap().clone()).unwrap(),
        )
        .icon(
            "update_app_setting",
            "刷新数据库",
            Image::from_path(APP_PIC_PATH.get("refresh").unwrap().clone()).unwrap(),
        )
        .icon(
            "retry_register_shortcut",
            "重新注册快捷键",
            Image::from_path(APP_PIC_PATH.get("register").unwrap().clone()).unwrap(),
        )
        .icon(
            "switch_game_mode",
            "开启游戏模式",
            Image::from_path(APP_PIC_PATH.get("game").unwrap().clone()).unwrap(),
        )
        .icon(
            "exit_program",
            "退出程序",
            Image::from_path(APP_PIC_PATH.get("exit").unwrap().clone()).unwrap(),
        )
        .build()
        .unwrap();
    let t = APP_PIC_PATH.get("tray_icon").unwrap();
    let icon_path = t.value();
    let tray_icon = TrayIconBuilder::new()
        .menu(&menu)
        .icon(Image::from_path(icon_path).unwrap())
        .tooltip(format!("ZeroLaunch-rs v{}", APP_VERSION.clone()))
        .show_menu_on_left_click(false)
        .build(handle)
        .unwrap();
    tray_icon.on_menu_event(move |app_handle, event| {
        let event_id = MenuEventId::from(event.id().as_ref());
        match event_id {
            MenuEventId::ShowSettingWindow => {
                if let Err(e) = show_setting_window() {
                    warn!("Failed to show setting window: {:?}", e);
                }
            }
            MenuEventId::ExitProgram => {
                tauri::async_runtime::block_on(async move {
                    save_config_to_file(false).await;
                    let storage_manager =
                        ServiceLocator::get_state().get_storage_manager().unwrap();
                    storage_manager.upload_all_file_force().await;
                });
                app_handle.exit(0);
            }
            MenuEventId::UpdateAppSetting => tauri::async_runtime::block_on(async {
                update_app_setting().await;
            }),
            MenuEventId::RegisterShortcut => {
                let state = ServiceLocator::get_state();
                let target_game_mode = state.get_game_mode();
                if target_game_mode {
                    notify("ZeroLaunch-rs", "请先关闭游戏模式");
                    return;
                }
                let shortcut_manager = state.get_shortcut_manager().unwrap();
                let _ = shortcut_manager.register_all_shortcuts();
                notify("ZeroLaunch-rs", "已完成注册注册");
            }
            MenuEventId::SwitchGameMode => {
                let temp = menu.get("switch_game_mode").unwrap();
                let item = temp.as_menuitem().unwrap();

                let state = ServiceLocator::get_state();
                let shortcut_manager = state.get_shortcut_manager().unwrap();
                let target_game_mode = !state.get_game_mode();
                state.set_game_mode(target_game_mode);
                if target_game_mode {
                    let _ = shortcut_manager.unregister_all_shortcut();
                } else {
                    let _ = shortcut_manager.register_all_shortcuts();
                }
                if target_game_mode {
                    let _ = item.set_text("关闭游戏模式");
                } else {
                    let _ = item.set_text("开启游戏模式");
                }
            }
            MenuEventId::Unknown(id) => {
                warn!("Unknown menu event: {}", id);
            }
        }
        debug!("Menu ID: {}", event.id().0);
    });

    let state = app.state::<Arc<AppState>>();
    state.set_tray_icon(Arc::new(tray_icon));

    app.on_tray_icon_event(|app_handle, event| match event {
        TrayIconEvent::DoubleClick { .. } => {
            handle_pressed(&app_handle);
        }
        _ => {}
    });
}
