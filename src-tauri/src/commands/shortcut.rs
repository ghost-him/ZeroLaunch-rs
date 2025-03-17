use crate::core::keyboard_listener::Shortcut;
use crate::core::keyboard_listener::ShortcutUnit;
use crate::handle_pressed;
use crate::AppState;
use std::sync::Arc;
/// 用于处理快捷键相关的命令
///
use tauri::Runtime;

#[tauri::command]
pub async fn get_all_shortcut<R: Runtime>(
    app: tauri::AppHandle<R>,
    window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<ShortcutUnit>, String> {
    let shortcut_manager = state.get_shortcut_manager().unwrap();
    let data = shortcut_manager.get_all_shortcuts();
    println!("{:?}", data);
    Ok(data)
}

#[tauri::command]
pub async fn delete_shortcut<R: Runtime>(
    app: tauri::AppHandle<R>,
    window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
    id: String,
) -> Result<(), String> {
    println!("取消注册");
    let shortcut_manager = state.get_shortcut_manager().unwrap();
    shortcut_manager.unregister_shortcut(&id)
}

#[tauri::command]
pub async fn register_shortcut<R: Runtime>(
    app: tauri::AppHandle<R>,
    window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
    id: String,
    shortcut: Shortcut,
) -> Result<(), String> {
    let shortcut_manager = state.get_shortcut_manager().unwrap();
    println!("收到shortcut:{:?}", &shortcut);
    shortcut_manager.register_shortcut(id, shortcut, |handle| {
        handle_pressed(handle);
    })
}
