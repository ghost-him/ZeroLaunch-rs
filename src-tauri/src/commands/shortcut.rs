use crate::AppState;
use std::sync::Arc;
use tauri::Runtime;

#[tauri::command]
pub async fn command_unregister_all_shortcut<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    if state.get_game_mode() {
        return Err("请关闭游戏模式后再更改".to_string());
    }
    let shortcut_manager = state.get_shortcut_manager().unwrap();
    shortcut_manager.unregister_all_shortcut()
}

#[tauri::command]
pub async fn command_register_all_shortcut<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let shortcut_manager = state.get_shortcut_manager().unwrap();
    shortcut_manager.register_all_shortcuts()
}
