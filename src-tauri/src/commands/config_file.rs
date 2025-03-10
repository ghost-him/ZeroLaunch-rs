use crate::modules::config::config_manager::PartialConfig;
use crate::save_config_to_file;
use crate::storage::config::PartialLocalConfig;
use crate::AppState;
use std::sync::Arc;
use tauri::Emitter;
use tauri::Runtime;
use tracing::debug;

/// 更新程序管理器的路径配置
#[tauri::command]
pub async fn command_save_remote_config<R: Runtime>(
    app: tauri::AppHandle<R>,
    state: tauri::State<'_, Arc<AppState>>,
    partial_config: PartialConfig,
) -> Result<(), String> {
    let runtime_config = state.get_runtime_config().unwrap();
    debug!("{:?}", partial_config);
    runtime_config.update(partial_config);
    save_config_to_file(true).await;
    app.emit("update_search_bar_window", "").unwrap();
    Ok(())
}

#[tauri::command]
pub async fn command_load_local_config<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<PartialLocalConfig, String> {
    let storage_manager = state.get_storage_manager().unwrap();
    Ok(storage_manager.to_partial().await)
}

#[tauri::command]
pub async fn command_save_local_config<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
    partial_config: PartialLocalConfig,
) -> Result<(), String> {
    let storage_manager = state.get_storage_manager().unwrap();
    storage_manager.update(partial_config).await;
    Ok(())
}
