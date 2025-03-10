use crate::core::image_processor::ImageProcessor;
use crate::state::app_state::AppState;
use crate::utils::service_locator::ServiceLocator;
use serde::{Deserialize, Serialize};
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
pub async fn initialize_search_window() -> SearchBarInit {
    let state = ServiceLocator::get_state();
    let runtime_config = state.get_runtime_config().unwrap();
    let result_item_count = runtime_config.get_app_config().get_search_result_count();
    SearchBarInit {
        result_item_count: result_item_count,
    }
}

#[tauri::command]
pub async fn update_search_bar_window<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<SearchBarUpdate, String> {
    let runtime_config = state.get_runtime_config().unwrap();
    let app_config = runtime_config.get_app_config();
    let ui_config = runtime_config.get_ui_config();
    Ok(SearchBarUpdate {
        search_bar_placeholder: app_config.get_search_bar_placeholder(),
        selected_item_color: ui_config.get_selected_item_color(),
        item_font_color: ui_config.get_item_font_color(),
        tips: app_config.get_tips(),
        search_bar_font_color: ui_config.get_search_bar_font_color(),
        search_bar_background_color: ui_config.get_search_bar_background_color(),
        item_font_size: ui_config.get_item_font_size(),
        search_bar_font_size: ui_config.get_search_bar_font_size(),
    })
}

#[tauri::command]
pub async fn get_background_picture<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<u8>, String> {
    let storage_manager = state.get_storage_manager().unwrap();
    let result = storage_manager
        .download_file_bytes("background.png".to_string())
        .await;
    Ok(result)
}

#[tauri::command]
pub async fn get_remote_config_dir<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let storage_manager = state.get_storage_manager().unwrap();
    let path = storage_manager.get_target_dir_path().await;
    Ok(path)
}

#[tauri::command]
pub async fn select_background_picture<R: Runtime>(
    app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
    path: String,
) -> Result<(), String> {
    let content: Vec<u8> = ImageProcessor::load_image_from_path(path).await;
    let storage_manager = state.get_storage_manager().unwrap();
    storage_manager
        .upload_file_bytes("background.png".to_string(), content)
        .await;
    app.emit("update_search_bar_window", "").unwrap();
    Ok(())
}

#[tauri::command]
pub async fn get_dominant_color<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    path: String,
) -> Result<String, String> {
    let content = ImageProcessor::load_image_from_path(path).await;
    let ret = ImageProcessor::get_dominant_color(content).await.unwrap();
    Ok(format!("rgba({}, {}, {}, 0.8)", ret.0, ret.1, ret.2))
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
    setting_window.set_focus().unwrap();
    hide_window().unwrap();
    Ok(())
}
