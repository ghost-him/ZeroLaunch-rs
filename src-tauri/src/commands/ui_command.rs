use crate::core::image_processor::ImageIdentity;
use crate::core::image_processor::ImageProcessor;
use crate::modules::config::app_config::PartialAppConfig;
use crate::modules::config::default::APP_PIC_PATH;
use crate::modules::config::ui_config::PartialUiConfig;
use crate::modules::shortcut_manager::shortcut_config::PartialShortcutConfig;
use crate::state::app_state::AppState;
use crate::utils::service_locator::ServiceLocator;
use crate::utils::ui_controller::handle_focus_lost;
use std::sync::Arc;
use tauri::image::Image;
use tauri::Emitter;
use tauri::Manager;
use tauri::Runtime;

#[tauri::command]
pub async fn update_search_bar_window<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(PartialAppConfig, PartialUiConfig, PartialShortcutConfig), String> {
    let runtime_config = state.get_runtime_config();
    let app_config = runtime_config.get_app_config();
    let ui_config = runtime_config.get_ui_config();
    let shortcut_config = runtime_config.get_shortcut_config();
    Ok((
        app_config.to_partial(),
        ui_config.to_partial(),
        shortcut_config.to_partial(),
    ))
}

#[tauri::command]
pub async fn get_background_picture<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<u8>, String> {
    let storage_manager = state.get_storage_manager();
    if let Some(data) = storage_manager
        .download_file_bytes("background.png".to_string())
        .await
    {
        return Ok(data);
    } else {
        storage_manager
            .upload_file_bytes("background.png".to_string(), Vec::new())
            .await;
    }
    Ok(Vec::new())
}

#[tauri::command]
pub async fn get_remote_config_dir<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let storage_manager = state.get_storage_manager();
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
    let path = ImageIdentity::File(path);
    let content: Vec<u8> = ImageProcessor::load_image(&path).await;
    let storage_manager = state.get_storage_manager();
    storage_manager
        .upload_file_bytes("background.png".to_string(), content)
        .await;
    if let Err(e) = app.emit("update_search_bar_window", "") {
        return Err(format!("Failed to emit update event: {:?}", e));
    }
    Ok(())
}

#[tauri::command]
pub async fn get_dominant_color<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    path: String,
) -> Result<String, String> {
    let path = ImageIdentity::File(path);
    let content = ImageProcessor::load_image(&path).await;
    let ret = match ImageProcessor::get_dominant_color(content).await {
        Ok(color) => color,
        Err(e) => return Err(format!("Failed to get dominant color: {:?}", e)),
    };
    Ok(format!("rgba({}, {}, {}, 0.8)", ret.0, ret.1, ret.2))
}

#[cfg(target_arch = "x86_64")]
#[tauri::command]
pub async fn get_everything_icon<R: Runtime>(
    _app: tauri::AppHandle<R>,
    state: tauri::State<'_, Arc<AppState>>,
    path: String,
) -> Result<Vec<u8>, String> {
    let icon_manager = state.get_icon_manager();
    let icon_data = icon_manager.get_everything_icon(path).await;
    Ok(icon_data)
}

#[cfg(not(target_arch = "x86_64"))]
#[tauri::command]
pub async fn get_everything_icon<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _state: tauri::State<'_, Arc<AppState>>,
    _path: String,
) -> Result<Vec<u8>, String> {
    Ok(Vec::new())
}

/// 隐藏窗口
#[tauri::command]
pub fn hide_window() -> Result<(), String> {
    let state = ServiceLocator::get_state();
    let main_window = match state.get_main_handle().get_webview_window("main") {
        Some(window) => window,
        None => return Err("Failed to get main window".to_string()),
    };
    handle_focus_lost(Arc::new(main_window));
    Ok(())
}

/// 展示设置窗口
#[tauri::command]
pub fn show_setting_window() -> Result<(), String> {
    let state = ServiceLocator::get_state();
    let setting_window = match state.get_main_handle().get_webview_window("setting_window") {
        Some(window) => window,
        None => return Err("Failed to get setting window".to_string()),
    };
    let _ = setting_window.unminimize();
    if let Err(e) = setting_window.show() {
        return Err(format!("Failed to show setting window: {:?}", e));
    }
    if let Err(e) = setting_window.set_focus() {
        return Err(format!("Failed to set focus on setting window: {:?}", e));
    }
    if let Err(e) = hide_window() {
        return Err(format!("Failed to hide window: {:?}", e));
    }
    Ok(())
}

/// 显示欢迎窗口
#[tauri::command]
pub async fn show_welcome_window<R: Runtime>(app: tauri::AppHandle<R>) -> Result<(), String> {
    use std::sync::Arc;
    use tauri::{LogicalSize, WebviewUrl, WebviewWindowBuilder};

    // 先关闭已存在的欢迎窗口（如果有的话）
    if let Some(existing_window) = app.get_webview_window("welcome") {
        let _ = existing_window.close();
    }

    // 创建新的欢迎窗口
    let welcome_result =
        WebviewWindowBuilder::new(&app, "welcome", WebviewUrl::App("/welcome".into()))
            .title("欢迎使用 ZeroLaunch-rs!")
            .visible(true)
            .drag_and_drop(false)
            .build();

    match welcome_result {
        Ok(welcome_window) => {
            if let Err(e) = welcome_window.set_size(LogicalSize::new(950, 500)) {
                return Err(format!("Failed to set welcome window size: {:?}", e));
            }

            // 监听窗口关闭事件，确保窗口关闭时清除内存
            let welcome_arc = Arc::new(welcome_window);
            let welcome_for_event = welcome_arc.clone();
            welcome_for_event.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { .. } = event {
                    // 窗口关闭时，窗口会自动从内存中清除
                    // 这里可以添加额外的清理逻辑（如果需要的话）
                }
            });
        }
        Err(e) => {
            return Err(format!("Failed to create welcome window: {:?}", e));
        }
    }

    Ok(())
}

/// 用于更改系统托盘图标的颜色

#[tauri::command]
pub async fn command_change_tray_icon<R: Runtime>(
    _app: tauri::AppHandle<R>,
    _window: tauri::Window<R>,
    state: tauri::State<'_, Arc<AppState>>,
    is_dark: bool,
) -> Result<(), String> {
    let key = {
        if is_dark {
            "tray_icon_white"
        } else {
            "tray_icon"
        }
    };

    let icon_path = match APP_PIC_PATH.get(key) {
        Some(path) => path,
        None => return Err(format!("Icon path not found for key: {}", key)),
    };
    let tray_icon = state.get_tray_icon();
    let image = match Image::from_path(icon_path.value()) {
        Ok(img) => img,
        Err(e) => return Err(format!("Failed to load icon image: {:?}", e)),
    };
    if let Err(e) = tray_icon.set_icon(Some(image)) {
        return Err(format!("error: {:?}", e));
    }
    Ok(())
}
