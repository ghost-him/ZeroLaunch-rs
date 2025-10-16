use crate::program_manager::unit::SearchTestResult;
use crate::state::app_state::AppState;
use std::sync::Arc;
/// 这个页面存放用于测试的代码
use tauri::Runtime;

#[tauri::command]
pub async fn test_search_algorithm<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
    search_text: String,
) -> Result<Vec<SearchTestResult>, String> {
    // 获取初始化读锁，确保初始化已完成
    let initialization_lock = state.get_initialization_lock();
    let _init_guard = initialization_lock.read().await;

    let program_manager = state.get_program_manager();
    Ok(program_manager.test_search_algorithm(&search_text).await)
}

#[tauri::command]
pub async fn test_search_algorithm_time<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(f64, f64, f64), String> {
    // 获取初始化读锁，确保初始化已完成
    let initialization_lock = state.get_initialization_lock();
    let _init_guard = initialization_lock.read().await;

    let program_manager = state.get_program_manager();
    Ok(program_manager.test_search_algorithm_time().await)
}

#[tauri::command]
pub async fn test_index_app_time<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<f64, String> {
    // 获取初始化读锁，确保初始化已完成
    let initialization_lock = state.get_initialization_lock();
    let _init_guard = initialization_lock.read().await;

    let program_manager = state.get_program_manager();
    let time = program_manager.get_program_loader_loading_time().await;
    Ok(time)
}

#[tauri::command]
pub async fn get_search_keys<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
    show_name: String,
) -> Result<Vec<String>, String> {
    // 获取初始化读锁，确保初始化已完成
    let initialization_lock = state.get_initialization_lock();
    let _init_guard = initialization_lock.read().await;

    let program_manager = state.get_program_manager();
    let search_keywords = program_manager.get_search_keywords(&show_name).await;
    Ok(search_keywords)
}
