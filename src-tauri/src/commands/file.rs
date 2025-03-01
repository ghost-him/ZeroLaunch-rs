use crate::modules::config::config_manager::PartialConfig;
use crate::save_config_to_file;
use crate::AppState;

use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use tauri::Emitter;
use tauri::Runtime;
use tracing::debug;

/// 更新程序管理器的路径配置
#[tauri::command]
pub fn save_config<R: Runtime>(
    app: tauri::AppHandle<R>,
    state: tauri::State<'_, Arc<AppState>>,
    partial_config: PartialConfig,
) -> Result<(), String> {
    let runtime_config = state.get_runtime_config().unwrap();
    debug!("{:?}", partial_config);
    runtime_config.update(partial_config);
    save_config_to_file(true);
    app.emit("update_search_bar_window", "").unwrap();
    Ok(())
}
