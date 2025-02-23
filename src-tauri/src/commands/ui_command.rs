use crate::commands::file::copy_background_picture;
use crate::commands::utils::get_background_picture_path;
use crate::modules::storage::utils::is_writable_directory;
use crate::modules::storage::utils::read_or_create_bytes;
use crate::state::app_state::AppState;
use crate::update_app_setting;
use crate::LocalConfig;
use crate::LOCAL_CONFIG_PATH;
use crate::{
    modules::ui_controller::controller::{get_item_size, get_window_size},
    utils::service_locator::ServiceLocator,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::Emitter;
use tauri::Manager;
use tauri::Runtime;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SearchBarInit {
    window_size: Vec<usize>,
    item_size: Vec<usize>,
    window_scale_factor: f64,
    result_item_count: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SearchBarUpdate {
    search_bar_placeholder: String,
    selected_item_color: String,
    item_font_color: String,
}

#[tauri::command]
pub fn initialize_search_window() -> SearchBarInit {
    let (item_width, item_height) = get_item_size();
    let (window_width, window_height) = get_window_size();

    let state = ServiceLocator::get_state();
    let runtime_config = state.get_runtime_config().unwrap();

    let window_scale_factor = runtime_config
        .get_window_state()
        .get_sys_window_scale_factor();
    let result_item_count = runtime_config.get_app_config().get_search_result_count();
    SearchBarInit {
        item_size: vec![item_width, item_height],
        window_size: vec![window_width, window_height],
        window_scale_factor: window_scale_factor,
        result_item_count: result_item_count,
    }
}

#[tauri::command]
pub fn update_search_bar_window<R: Runtime>(
    app: tauri::AppHandle<R>,
    window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
) -> SearchBarUpdate {
    let runtime_config = state.get_runtime_config().unwrap();
    let app_config = runtime_config.get_app_config();
    let ui_config = runtime_config.get_ui_config();
    SearchBarUpdate {
        search_bar_placeholder: app_config.get_search_bar_placeholder(),
        selected_item_color: ui_config.get_selected_item_color(),
        item_font_color: ui_config.get_item_font_color(),
    }
}

#[tauri::command]
pub async fn get_background_picture<R: Runtime>(
    app: tauri::AppHandle<R>,
    window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<u8>, String> {
    let target_path = get_background_picture_path();
    read_or_create_bytes(&target_path, None)
}

#[tauri::command]
pub async fn change_remote_config_dir<R: Runtime>(
    app: tauri::AppHandle<R>,
    window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
    config_dir: String,
) -> Result<(), String> {
    // 先判断是不是正确的文件夹
    if !is_writable_directory(&config_dir) {
        return Err("当前的文件夹无法创建新的文件，请更改文件夹的权限或更改目标文件夹".to_string());
    }
    let result = LocalConfig {
        remote_config_path: config_dir.clone(),
    };
    let data = serde_json::to_string(&result).unwrap();

    let path = LOCAL_CONFIG_PATH.clone();
    std::fs::write(path, data).unwrap();

    state.set_remote_config_dir_path(config_dir.clone());
    let runtime_config = state.get_runtime_config().unwrap();
    runtime_config.load_from_remote_config_path(Some(config_dir));

    update_app_setting();
    app.emit("update_search_bar_window", "").unwrap();
    Ok(())
}

#[tauri::command]
pub async fn select_background_picture<R: Runtime>(
    app: tauri::AppHandle<R>,
    window: tauri::Window<R>,
    path: String,
) -> Result<(), String> {
    let result = copy_background_picture(path);
    app.emit("update_search_bar_window", "").unwrap();
    result
}

/// 隐藏窗口
#[tauri::command]
pub fn hide_window() -> Result<(), String> {
    let state = ServiceLocator::get_state();
    let main_window = state
        .get_main_handle()
        .unwrap()
        .get_webview_window("main")
        .unwrap();
    main_window.hide().unwrap();
    Ok(())
}

/// 展示设置窗口
#[tauri::command]
pub fn show_setting_window() -> Result<(), String> {
    let state = ServiceLocator::get_state();
    let setting_window = state
        .get_main_handle()
        .unwrap()
        .get_webview_window("setting_window")
        .unwrap();
    setting_window.show().unwrap();
    hide_window().unwrap();
    Ok(())
}
