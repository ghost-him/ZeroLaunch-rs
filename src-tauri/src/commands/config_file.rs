//use crate::core::storage::onedrive::get_onedrive_refresh_token;
use crate::core::storage::storage_manager::check_validation;
use crate::modules::config::config_manager::PartialRuntimeConfig;
use crate::modules::config::default::REMOTE_CONFIG_DEFAULT;
use crate::modules::config::load_string_to_runtime_config_;
use crate::save_config_to_file;
use crate::storage::config::PartialLocalConfig;
use crate::utils::service_locator::ServiceLocator;
use crate::AppState;
use crate::REMOTE_CONFIG_NAME;
use std::sync::Arc;
use tauri::Emitter;
use tauri::Manager;
use tauri::Runtime;
use tracing::error;

/// 更新程序管理器的路径配置
#[tauri::command]
pub async fn command_save_remote_config<R: Runtime>(
    _app: tauri::AppHandle<R>,
    state: tauri::State<'_, Arc<AppState>>,
    partial_config: PartialRuntimeConfig,
) -> Result<(), String> {
    use tracing::info;

    info!("💾 开始保存远程配置");
    println!("收到的远程配置: {:?}", partial_config);

    let runtime_config = state.get_runtime_config();

    runtime_config.update(partial_config);
    info!("✅ 运行时配置已更新");

    save_config_to_file(true).await;
    info!("💾 远程配置保存完成");

    Ok(())
}

#[tauri::command]
pub async fn command_load_local_config<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<PartialLocalConfig, String> {
    use tracing::info;

    info!("📂 开始加载本地配置");

    let storage_manager = state.get_storage_manager();

    let config = storage_manager.to_partial().await;
    info!("✅ 本地配置加载完成");

    Ok(config)
}

#[tauri::command]
pub async fn command_save_local_config<R: Runtime>(
    app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
    partial_config: PartialLocalConfig,
) -> Result<(), String> {
    use tracing::{debug, info, warn};

    info!("💾 开始保存本地配置");

    let storage_manager = state.get_storage_manager();

    debug!("📤 强制上传所有文件");
    storage_manager.upload_all_file_force().await;

    debug!("🔄 更新存储管理器配置");
    storage_manager.update(partial_config).await;

    let runtime_config = state.get_runtime_config();

    debug!("📥 获取远程配置数据");
    let remote_config_data = {
        if let Some(data) = storage_manager
            .download_file_str(REMOTE_CONFIG_NAME.to_string())
            .await
        {
            debug!("✅ 从远程下载配置成功");
            data
        } else {
            debug!("📤 远程配置不存在，上传默认配置");
            storage_manager
                .upload_file_str(
                    REMOTE_CONFIG_NAME.to_string(),
                    REMOTE_CONFIG_DEFAULT.clone(),
                )
                .await;
            REMOTE_CONFIG_DEFAULT.clone()
        }
    };

    debug!("🔄 加载并更新运行时配置");
    let partial_config = load_string_to_runtime_config_(&remote_config_data);
    runtime_config.update(partial_config);

    debug!("⚙️ 更新应用设置");
    let state = ServiceLocator::get_state();
    state.get_refresh_scheduler().trigger_refresh();

    let setting_window = match app.get_webview_window("setting_window") {
        Some(window) => window,
        None => {
            warn!("❌ 获取设置窗口失败");
            return Err("Failed to get setting window".to_string());
        }
    };

    if let Err(e) = setting_window.emit("emit_update_setting_window_config", "") {
        error!("向 setting_window 发送信号失败: {:?}", e);
    } else {
        debug!("📡 设置窗口更新信号发送成功");
    }

    info!("✅ 本地配置保存完成");
    Ok(())
}

#[tauri::command]
pub async fn command_check_validation<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    partial_config: PartialLocalConfig,
) -> Result<Option<PartialLocalConfig>, String> {
    Ok(check_validation(partial_config).await)
}

// #[tauri::command]
// pub async fn command_get_onedrive_refresh_token<R: Runtime>(
//     app: tauri::AppHandle<R>,
//     window: tauri::Window<R>,
// ) -> Result<String, String> {
//     get_onedrive_refresh_token(window).await
// }
