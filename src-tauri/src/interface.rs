use crate::config::AppConfig;
use crate::program_manager::PROGRAM_MANAGER;
/// 用于后端向前端传输数据，或者是前端向后端数据
///
///
use crate::RuntimeConfig;
use crate::Singleton;
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
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SearchBarUpdate {
    search_bar_placeholder: String,
}

#[tauri::command]
pub fn init_search_bar_window() -> SearchBarInit {
    let instance = RuntimeConfig::instance();
    let runtime_config = instance.lock().unwrap();

    let (item_width, item_height) = runtime_config.get_item_size();
    let (window_width, window_height) = runtime_config.get_window_size();

    SearchBarInit {
        item_size: vec![item_width, item_height],
        window_size: vec![window_width, window_height],
        window_scale_factor: runtime_config.get_window_scale_factor(),
    }
}

#[tauri::command]
pub fn update_search_bar_window() -> SearchBarUpdate {
    let instance = RuntimeConfig::instance();
    let runtime_config = instance.lock().unwrap();
    let app_config = runtime_config.get_app_config();
    SearchBarUpdate {
        search_bar_placeholder: app_config.search_bar_placeholder.clone(),
    }
}

#[derive(Serialize, Debug)]
pub struct SearchResult(u64, String);

/// 处理前端发来的消息
#[tauri::command]
pub fn handle_search_text(search_text: String) -> Vec<SearchResult> {
    // 处理消息
    let manager = PROGRAM_MANAGER.lock().unwrap();
    let results = manager.update(&search_text, 4);
    // 解锁
    drop(manager);
    let mut ret = Vec::new();
    for item in results {
        ret.push(SearchResult(item.0, item.1));
    }
    println!("{:?}", ret);
    ret
}

/// 隐藏窗口
#[tauri::command]
pub fn hide_window<R: Runtime>(app: tauri::AppHandle<R>) -> Result<(), String> {
    let main_window = Arc::new(app.get_webview_window("main").unwrap());
    main_window.hide().unwrap();
    Ok(())
}

/// 展示设置窗口
#[tauri::command]
pub fn show_setting_window<R: Runtime>(app: tauri::AppHandle<R>) -> Result<(), String> {
    let setting_window = app.get_webview_window("setting_window").unwrap();
    setting_window.show().unwrap();
    hide_window(app).unwrap();
    Ok(())
}

/// 获得程序的设置界面
#[tauri::command]
pub fn get_app_config() -> Result<AppConfig, String> {
    let instance = RuntimeConfig::instance();
    let config = instance.lock().unwrap();
    let app_config = config.get_app_config();
    Ok(app_config.clone())
}

/// 保存程序的设置
#[tauri::command]
pub async fn save_app_config(app: tauri::AppHandle, app_config: AppConfig) -> Result<(), String> {
    let instance = RuntimeConfig::instance();
    let mut config: std::sync::MutexGuard<'_, RuntimeConfig> = instance.lock().unwrap();
    println!("收到配置");
    config.save_app_config(app_config);
    app.emit("update_search_bar_window", "").unwrap();
    Ok(())
}
