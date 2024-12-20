use super::save_config_to_file;
use crate::config::{AppConfig, LOCAL_CONFIG_PATH};
use crate::config::{LocalConfig, REMOTE_CONFIG_DIR_PATH};
use crate::program_manager::PROGRAM_MANAGER;
use crate::utils::copy_background_picture;
use crate::utils::is_writable_directory;
use crate::Singleton;
/// 用于后端向前端传输数据，或者是前端向后端数据
///
///
use crate::{update_app_setting, RuntimeConfig};
use rayon::iter::ParallelIterator;
use serde::{Deserialize, Serialize};
use std::fs::write;
use std::sync::Arc;
use tauri::async_runtime::spawn_blocking;
use tauri::Emitter;
use tauri::Manager;
use tauri::Runtime;
use tracing::debug;
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
    pub history_launch_time: u64,
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
        result_item_count: runtime_config.get_app_config().search_result_count,
    }
}

/// 更新搜索窗口
#[tauri::command]
pub fn update_search_bar_window() -> SearchBarUpdate {
    let instance = RuntimeConfig::instance();
    let runtime_config = instance.lock().unwrap();
    let app_config = runtime_config.get_app_config();
    let ui_config = runtime_config.get_ui_config();
    SearchBarUpdate {
        search_bar_placeholder: app_config.search_bar_placeholder.clone(),
        selected_item_color: ui_config.selected_item_color.clone(),
        item_font_color: ui_config.item_font_color.clone(),
    }
}

#[derive(Serialize, Debug)]
pub struct SearchResult(u64, String);

/// 处理前端发来的消息
#[tauri::command]
pub fn handle_search_text(search_text: String) -> Vec<SearchResult> {
    let instance = RuntimeConfig::instance();
    let runtime_config = instance.lock().unwrap();
    let result_count = runtime_config.get_app_config().search_result_count;
    drop(runtime_config);
    // 处理消息
    let manager = PROGRAM_MANAGER.lock().unwrap();
    let results = manager.update(&search_text, result_count);
    // 解锁
    drop(manager);
    let mut ret = Vec::new();
    for item in results {
        ret.push(SearchResult(item.0, item.1));
    }
    debug!("{:?}", ret);
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SettingWindowAppConfig {
    pub search_bar_placeholder: String,
    pub search_bar_no_result: String,
    pub is_auto_start: bool,
    pub is_silent_start: bool,
    pub search_result_count: u32,
    pub auto_refresh_time: u32,
    pub selected_item_color: String,
    pub item_font_color: String,
}

/// 获得程序的设置界面
#[tauri::command]
pub fn get_config() -> Result<SettingWindowAppConfig, String> {
    let instance = RuntimeConfig::instance();
    let config = instance.lock().unwrap();
    let app_config = config.get_app_config();
    let ui_config = config.get_ui_config();
    Ok(SettingWindowAppConfig {
        search_bar_placeholder: app_config.search_bar_placeholder.clone(),
        search_bar_no_result: app_config.search_bar_no_result.clone(),
        is_auto_start: app_config.is_auto_start,
        is_silent_start: app_config.is_silent_start,
        search_result_count: app_config.search_result_count,
        auto_refresh_time: app_config.auto_refresh_time,
        selected_item_color: ui_config.selected_item_color.clone(),
        item_font_color: ui_config.item_font_color.clone(),
    })
}

/// 保存程序的设置
#[tauri::command]
pub async fn save_app_config(
    app: tauri::AppHandle,
    app_config: SettingWindowAppConfig,
) -> Result<(), String> {
    let instance = RuntimeConfig::instance();
    let mut config: std::sync::MutexGuard<'_, RuntimeConfig> = instance.lock().unwrap();
    debug!("收到配置");
    config.save_app_config(AppConfig {
        search_bar_placeholder: app_config.search_bar_placeholder.clone(),
        search_bar_no_result: app_config.search_bar_no_result.clone(),
        is_auto_start: app_config.is_auto_start,
        is_silent_start: app_config.is_silent_start,
        search_result_count: app_config.search_result_count,
        auto_refresh_time: app_config.auto_refresh_time,
    });
    config.save_selected_item_color(app_config.selected_item_color);
    config.save_item_font_color(app_config.item_font_color);

    drop(config);
    save_config_to_file(true);
    app.emit("update_search_bar_window", "").unwrap();
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
pub fn save_path_config<R: Runtime>(
    app: tauri::AppHandle<R>,
    path_data: SettingWindowPathData,
) -> Result<(), String> {
    let instance = RuntimeConfig::instance();
    let mut config = instance.lock().unwrap();
    debug!("{:?}", path_data);
    config.save_path_config(path_data);
    drop(config);
    save_config_to_file(true);
    app.emit("update_search_bar_window", "").unwrap();
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
pub fn save_key_filter_data<R: Runtime>(
    app: tauri::AppHandle<R>,
    key_filter_data: Vec<KeyFilterData>,
) -> Result<(), String> {
    let instance = RuntimeConfig::instance();
    let mut config = instance.lock().unwrap();
    config.save_key_filter_config(key_filter_data);
    drop(config);
    save_config_to_file(true);
    app.emit("update_search_bar_window", "").unwrap();
    Ok(())
}

#[tauri::command]
pub fn get_program_info() -> Vec<ProgramInfo> {
    let mut manager = PROGRAM_MANAGER.lock().unwrap();
    let data = manager.get_program_infos();
    debug!("{:?}", data);
    drop(manager);
    let mut program_infos = Vec::new();
    for item in data {
        program_infos.push(ProgramInfo {
            name: item.0,
            is_uwp: item.1,
            bias: item.2,
            path: item.3,
            history_launch_time: item.4,
        })
    }
    program_infos
}

#[tauri::command]
pub async fn launch_program<R: Runtime>(
    app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    program_guid: u64,
    is_admin_required: bool,
) -> Result<(), String> {
    let mut manager = PROGRAM_MANAGER.lock().unwrap();
    hide_window(app).unwrap();
    manager.launch_program(program_guid, is_admin_required);
    // 开一个新的线程来完成保存文件
    spawn_blocking(|| {
        save_config_to_file(false);
    });
    Ok(())
}

#[tauri::command]
pub async fn refresh_program<R: Runtime>(
    app: tauri::AppHandle<R>,
    window: tauri::Window<R>,
) -> Result<(), String> {
    super::update_app_setting();
    Ok(())
}

#[tauri::command]
pub async fn load_program_icon<R: Runtime>(
    app: tauri::AppHandle<R>,
    window: tauri::Window<R>,
    program_guid: u64,
) -> Result<Vec<u8>, String> {
    let manager = PROGRAM_MANAGER.lock().unwrap();
    let result = manager.get_icon(&program_guid);

    Ok(result)
}

#[tauri::command]
pub fn get_program_count() -> Result<usize, String> {
    let manager = PROGRAM_MANAGER.lock().unwrap();
    let result = manager.get_program_count();
    Ok(result)
}

#[tauri::command]
pub async fn get_file_info<R: Runtime>(
    app: tauri::AppHandle<R>,
    window: tauri::Window<R>,
) -> Result<Vec<String>, String> {
    let instance = RuntimeConfig::instance();
    let mut config = instance.lock().unwrap();
    let files = config.get_index_files();
    Ok(files)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct web_page_info {
    show_name: String,
    url: String,
}

#[tauri::command]
pub async fn save_custom_file_path<R: Runtime>(
    app: tauri::AppHandle<R>,
    window: tauri::Window<R>,
    web_pages: Vec<web_page_info>,
    file_paths: Vec<String>,
) -> Result<(), String> {
    let instance = RuntimeConfig::instance();
    let mut config = instance.lock().unwrap();

    let data = web_pages
        .iter()
        .map(|x| (x.show_name.clone(), x.url.clone()))
        .collect();
    config.save_web_pages_info(data);
    config.save_index_file_info(file_paths);
    drop(config);
    save_config_to_file(true);
    app.emit("update_search_bar_window", "").unwrap();
    Ok(())
}

#[tauri::command]
pub async fn get_web_pages_infos<R: Runtime>(
    app: tauri::AppHandle<R>,
    window: tauri::Window<R>,
) -> Vec<web_page_info> {
    let instance = RuntimeConfig::instance();
    let mut config = instance.lock().unwrap();

    let data = config.get_web_pages_info();
    data.into_iter()
        .map(|x| web_page_info {
            show_name: x.0,
            url: x.1,
        })
        .collect()
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

#[tauri::command]
pub async fn get_background_picture<R: Runtime>(
    app: tauri::AppHandle<R>,
    window: tauri::Window<R>,
) -> Result<Vec<u8>, String> {
    crate::utils::get_background_picture()
}

#[tauri::command]
pub async fn change_remote_config_dir<R: Runtime>(
    app: tauri::AppHandle<R>,
    window: tauri::Window<R>,
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
    let mut instance = REMOTE_CONFIG_DIR_PATH.write().unwrap();
    *instance = Some(config_dir);
    drop(instance);

    let instance = RuntimeConfig::instance();
    let mut runtime_config = instance.lock().unwrap();
    runtime_config.reload_runtime_config();
    drop(runtime_config);
    update_app_setting();
    app.emit("update_search_bar_window", "").unwrap();
    Ok(())
}

#[tauri::command]
pub async fn get_remote_config_dir<R: Runtime>(
    app: tauri::AppHandle<R>,
    window: tauri::Window<R>,
) -> String {
    let instance = REMOTE_CONFIG_DIR_PATH.read().unwrap();
    let result = (*instance).clone().unwrap();
    drop(instance);
    result
}
