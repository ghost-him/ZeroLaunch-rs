use super::update_app_setting;
use crate::config::AppConfig;
use crate::program_manager::PROGRAM_MANAGER;
/// 用于后端向前端传输数据，或者是前端向后端数据
///
///
use crate::RuntimeConfig;
use crate::Singleton;
use rdev::Key;
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

/// 用于传输路径相关的信息
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SettingWindowPathData {
    pub target_paths: Vec<String>,
    pub forbidden_paths: Vec<String>,
    pub forbidden_key: Vec<String>,
    pub is_scan_uwp_program: bool,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KeyFilterData {
    pub key: String,
    pub bias: f64,
    pub note: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProgramInfo {
    pub name: String,
    pub is_uwp: bool,
    pub bias: f64,
    pub path: String,
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

/// 更新搜索窗口
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
    drop(config);
    update_app_setting();
    Ok(())
}

/// 获得程序的设置界面中路径的设置标签页的内容
#[tauri::command]
pub fn get_path_config() -> Result<SettingWindowPathData, String> {
    let instance = RuntimeConfig::instance();
    let config = instance.lock().unwrap();
    let program_config = config.get_program_manager_config();
    Ok(SettingWindowPathData {
        target_paths: program_config.loader.target_paths.clone(),
        forbidden_paths: program_config.loader.forbidden_paths.clone(),
        forbidden_key: program_config.loader.forbidden_program_key.clone(),
        is_scan_uwp_program: program_config.loader.is_scan_uwp_programs,
    })
}

/// 更新程序管理器的路径配置
#[tauri::command]
pub fn save_path_config(path_data: SettingWindowPathData) -> Result<(), String> {
    let instance = RuntimeConfig::instance();
    let mut config = instance.lock().unwrap();
    config.save_path_config(path_data);
    drop(config);
    update_app_setting();
    Ok(())
}

#[tauri::command]
pub fn get_key_filter_data() -> Vec<KeyFilterData> {
    let instance = RuntimeConfig::instance();
    let config = instance.lock().unwrap();
    let program_config = config.get_program_manager_config();
    let mut result: Vec<KeyFilterData> = Vec::new();
    for item in &program_config.loader.program_bias {
        result.push(KeyFilterData {
            key: item.0.clone(),
            bias: item.1 .0,
            note: item.1 .1.clone(),
        });
    }
    result
}

#[tauri::command]
pub fn save_key_filter_data(key_filter_data: Vec<KeyFilterData>) -> Result<(), String> {
    let instance = RuntimeConfig::instance();
    let mut config = instance.lock().unwrap();
    config.save_key_filter_config(key_filter_data);
    drop(config);
    update_app_setting();
    Ok(())
}

#[tauri::command]
pub fn get_program_info() -> Vec<ProgramInfo> {
    let manager = PROGRAM_MANAGER.lock().unwrap();
    let data = manager.get_program_infos();
    println!("{:?}", data);
    drop(manager);
    let mut program_infos = Vec::new();
    for item in data {
        program_infos.push(ProgramInfo {
            name: item.0,
            is_uwp: item.1,
            bias: item.2,
            path: item.3,
        })
    }
    program_infos
}

#[tauri::command]
pub async fn launch_program<R: Runtime>(
    app: tauri::AppHandle<R>,
    window: tauri::Window<R>,
    program_guid: u64,
    is_admin_required: bool,
) -> Result<(), String> {
    let manager = PROGRAM_MANAGER.lock().unwrap();
    manager.launch_program(program_guid, is_admin_required);
    Ok(())
}

#[tauri::command]
async fn refresh_program<R: Runtime>(
    app: tauri::AppHandle<R>,
    window: tauri::Window<R>,
) -> Result<(), String> {
    super::update_app_setting();
    Ok(())
}
