use crate::commands::utils::get_background_picture_path;
use crate::core::image_processor::ImageProcessor;
use crate::modules::storage::utils::is_writable_directory;
use crate::modules::storage::utils::read_or_create_bytes;
use crate::state::app_state::AppState;
use crate::update_app_setting;
use crate::utils::service_locator::ServiceLocator;
use crate::LocalConfig;
use crate::LOCAL_CONFIG_PATH;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use tauri::Emitter;
use tauri::Manager;
use tauri::Runtime;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SearchBarInit {
    result_item_count: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SearchBarUpdate {
    search_bar_placeholder: String,
    selected_item_color: String,
    item_font_color: String,
    tips: String,
    search_bar_font_color: String,
    search_bar_background_color: String,
    item_font_size: f64,
    search_bar_font_size: f64,
}

#[tauri::command]
pub fn initialize_search_window() -> SearchBarInit {
    let state = ServiceLocator::get_state();
    let runtime_config = state.get_runtime_config().unwrap();
    let result_item_count = runtime_config.get_app_config().get_search_result_count();
    SearchBarInit {
        result_item_count: result_item_count,
    }
}

#[tauri::command]
pub fn update_search_bar_window<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
) -> SearchBarUpdate {
    let runtime_config = state.get_runtime_config().unwrap();
    let app_config = runtime_config.get_app_config();
    let ui_config = runtime_config.get_ui_config();
    SearchBarUpdate {
        search_bar_placeholder: app_config.get_search_bar_placeholder(),
        selected_item_color: ui_config.get_selected_item_color(),
        item_font_color: ui_config.get_item_font_color(),
        tips: app_config.get_tips(),
        search_bar_font_color: ui_config.get_search_bar_font_color(),
        search_bar_background_color: ui_config.get_search_bar_background_color(),
        item_font_size: ui_config.get_item_font_size(),
        search_bar_font_size: ui_config.get_search_bar_font_size(),
    }
}

#[tauri::command]
pub async fn get_background_picture<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    _state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<u8>, String> {
    let target_path = get_background_picture_path();
    read_or_create_bytes(&target_path, None)
}

#[tauri::command]
pub async fn change_remote_config_dir<R: Runtime>(
    app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
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
pub async fn get_remote_config_dir<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<String, String> {
    Ok(state.get_remote_config_dir_path())
}

#[tauri::command]
pub async fn select_background_picture<R: Runtime>(
    app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    path: String,
) -> Result<(), String> {
    let content: Vec<u8> = ImageProcessor::load_image_from_path(&path);
    let target_path = get_background_picture_path();
    if let Ok(mut file) = File::create(target_path) {
        // 将所有字节写入文件
        let _ = file.write_all(&content);
    }
    app.emit("update_search_bar_window", "").unwrap();
    Ok(())
}

#[tauri::command]
pub async fn get_dominant_color<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    path: String,
) -> String {
    let content = ImageProcessor::load_image_from_path(&path);
    let ret = ImageProcessor::get_dominant_color(content).unwrap();
    format!("rgba({}, {}, {}, 0.8)", ret.0, ret.1, ret.2)
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
