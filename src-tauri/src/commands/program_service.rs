use crate::commands::ui_command::hide_window;
use crate::modules::config::config_manager::PartialConfig;
use crate::save_config_to_file;
use crate::state::app_state::AppState;
use crate::update_app_setting;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::async_runtime::spawn_blocking;
use tauri::Runtime;
use tracing::debug;
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
    let result = program_manager.get_icon(&program_guid);

    Ok(result)
}

#[tauri::command]
pub async fn get_program_count<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<usize, String> {
    let program_manager = state.get_program_manager().unwrap();
    let result = program_manager.get_program_count();
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

    let is_admin_required = ctrl && !shift;
    let open_exist_window = ctrl && shift;
    let mut result = false;
    // 只有当shift+ctrl同时按下时，才是唤醒程序
    if open_exist_window {
        println!("开始唤醒程序");
        result = program_manager.activate_target_program(program_guid);
        println!("结果：{}", result);
    }
    // 唤醒失败时启动新的程序
    let launch_new_on_failure = state
        .get_runtime_config()
        .unwrap()
        .get_app_config()
        .get_launch_new_on_failure();
    if (!result && launch_new_on_failure)
        || !open_exist_window
        || (!result && program_manager.is_uwp_program(program_guid))
    {
        println!("开启新的程序");
        // 启动新的程序
        program_manager.launch_program(program_guid, is_admin_required);
        // 开一个新的线程来完成保存文件
        spawn_blocking(|| {
            save_config_to_file(false);
        });
    }

    Ok(())
}

#[tauri::command]
pub fn get_program_info<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Vec<ProgramInfo> {
    let manager = state.get_program_manager().unwrap();
    let data = manager.get_program_infos();
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
    program_infos
}

#[tauri::command]
pub async fn refresh_program<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
) -> Result<(), String> {
    update_app_setting();
    Ok(())
}

/// 处理前端发来的消息
#[tauri::command]
pub fn handle_search_text<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
    search_text: String,
) -> Vec<SearchResult> {
    let runtime_config = state.get_runtime_config().unwrap();
    let result_count = runtime_config.get_app_config().get_search_result_count();
    // 处理消息
    let program_manager = state.get_program_manager().unwrap();
    let results = program_manager.update(&search_text, result_count);

    let mut ret = Vec::new();
    for item in results {
        ret.push(SearchResult(item.0, item.1));
    }
    debug!("{:?}", ret);
    ret
}

#[tauri::command]
pub async fn load_config<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<PartialConfig, String> {
    let runtime_config = state.get_runtime_config().unwrap();
    Ok(runtime_config.to_partial())
}
