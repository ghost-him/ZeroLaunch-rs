use crate::commands::ui_command::hide_window;
use crate::modules::config::config_manager::PartialRuntimeConfig;
use crate::modules::config::default::ICON_CACHE_DIR;
use crate::notify;
use crate::save_config_to_file;
use crate::state::app_state::AppState;
use crate::update_app_setting;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use tauri::Runtime;
use tracing::debug;
use tracing::warn;
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProgramInfo {
    pub name: String,
    pub is_uwp: bool,
    pub bias: f64,
    pub path: String,
    pub history_launch_time: u64,
}

/// 更新搜索窗口

#[derive(Serialize, Debug)]
pub struct SearchResult(u64, String);

#[tauri::command]
pub async fn load_program_icon<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
    program_guid: u64,
) -> Result<Vec<u8>, String> {
    let program_manager = state.get_program_manager().unwrap();
    let result = program_manager.get_icon(&program_guid).await;
    if result.is_empty() {
        warn!("id： {}， 获得图标失败！", program_guid);
    }

    Ok(result)
}

#[tauri::command]
pub async fn get_program_count<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<usize, String> {
    let program_manager = state.get_program_manager().unwrap();
    let result = program_manager.get_program_count().await;
    Ok(result)
}

#[tauri::command]
pub async fn launch_program<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
    program_guid: u64,
    ctrl: bool,
    shift: bool,
) -> Result<(), String> {
    let program_manager = state.get_program_manager().unwrap();
    hide_window().unwrap();

    let is_admin_required = ctrl;
    let open_exist_window = shift;
    let mut result = false;
    // 当shift按下时，唤醒程序
    if open_exist_window {
        result = program_manager.activate_target_program(program_guid).await;
    }
    // 唤醒失败时启动新的程序
    let launch_new_on_failure = state
        .get_runtime_config()
        .unwrap()
        .get_app_config()
        .get_launch_new_on_failure();
    if (!result && launch_new_on_failure)
        || !open_exist_window
        || (!result && program_manager.is_uwp_program(program_guid).await)
    {
        // 启动新的程序
        program_manager
            .launch_program(program_guid, is_admin_required)
            .await;
        // 保存文件
        save_config_to_file(false).await;
    }

    Ok(())
}

#[tauri::command]
pub async fn get_program_info<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<ProgramInfo>, String> {
    let manager = state.get_program_manager().unwrap();
    let data = manager.get_program_infos().await;
    debug!("{:?}", data);
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
    Ok(program_infos)
}

#[tauri::command]
pub async fn refresh_program<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
) -> Result<(), String> {
    update_app_setting().await;
    Ok(())
}

/// 处理前端发来的消息
#[tauri::command]
pub async fn handle_search_text<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
    search_text: String,
) -> Result<Vec<SearchResult>, String> {
    let runtime_config = state.get_runtime_config().unwrap();
    let result_count = runtime_config.get_app_config().get_search_result_count();
    // 处理消息
    let program_manager = state.get_program_manager().unwrap();
    let results = program_manager.update(&search_text, result_count).await;

    let mut ret = Vec::new();
    for item in results {
        ret.push(SearchResult(item.0, item.1));
    }
    debug!("{:?}", ret);
    Ok(ret)
}

/// 获得最近启动的程序
#[tauri::command]
pub async fn command_get_latest_launch_propgram(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<SearchResult>, String> {
    let runtime_config = state.get_runtime_config().unwrap();
    let result_count = runtime_config.get_app_config().get_search_result_count();
    // 处理消息
    let program_manager: Arc<crate::modules::program_manager::ProgramManager> =
        state.get_program_manager().unwrap();
    let results = program_manager
        .get_latest_launch_program(result_count)
        .await;
    let mut ret = Vec::new();
    for item in results {
        ret.push(SearchResult(item.0, item.1));
    }
    debug!("latest_launch_propgram: {:?}", ret);
    Ok(ret)
}

#[tauri::command]
pub async fn command_load_remote_config<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<PartialRuntimeConfig, String> {
    let runtime_config = state.get_runtime_config().unwrap();
    Ok(runtime_config.to_partial())
}

#[tauri::command]
pub async fn open_target_folder<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
    program_guid: u64,
) -> Result<bool, String> {
    let program_manager = state.get_program_manager().unwrap();
    hide_window().unwrap();
    let result = program_manager.open_target_folder(program_guid).await;
    if !result {
        notify("ZeroLaunch-rs", "打开文件夹失败，目标类型不支持");
        return Ok(false);
    }
    Ok(true)
}

#[tauri::command]
#[allow(clippy::zombie_processes)]
pub async fn command_open_icon_cache_dir<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
) -> Result<(), String> {
    let target_path = ICON_CACHE_DIR.clone();
    Command::new("explorer")
        .args([&target_path]) // 使用/select参数并指定完整文件路径
        .spawn()
        .unwrap();
    Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PathInfo {
    path_type: String, // "file"、"directory" 或 "error"
    original_path: String,
    parent_path: Option<String>,
    filename: Option<String>,
    error_message: Option<String>,
}

#[tauri::command]
pub fn command_get_path_info(path_str: String) -> PathInfo {
    let path = Path::new(&path_str);
    match fs::metadata(path) {
        Ok(metadata) => {
            if metadata.is_dir() {
                PathInfo {
                    path_type: "directory".to_string(),
                    original_path: path_str,
                    parent_path: None,
                    filename: None,
                    error_message: None,
                }
            } else if metadata.is_file() {
                let parent = path.parent().and_then(|p| p.to_str()).map(String::from);
                let filename = path.file_name().and_then(|n| n.to_str()).map(String::from);
                PathInfo {
                    path_type: "file".to_string(),
                    original_path: path_str,
                    parent_path: parent,
                    filename,
                    error_message: None,
                }
            } else {
                // 如果已检索到元数据，则不应经常发生，但要进行防御性处理
                PathInfo {
                    path_type: "error".to_string(),
                    original_path: path_str,
                    parent_path: None,
                    filename: None,
                    error_message: Some("Path is neither a file nor a directory.".to_string()),
                }
            }
        }
        Err(e) => PathInfo {
            path_type: "error".to_string(),
            original_path: path_str,
            parent_path: None,
            filename: None,
            error_message: Some(format!("Failed to access path metadata: {}", e)),
        },
    }
}
