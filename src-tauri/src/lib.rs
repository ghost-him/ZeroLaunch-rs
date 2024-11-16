pub mod config;
pub mod interface;
pub mod program_manager;
pub mod singleton;
pub mod ui_controller;
pub mod utils;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use crate::config::GLOBAL_APP_HANDLE;
use crate::interface::{
    get_app_config, get_key_filter_data, get_path_config, get_program_info, handle_search_text,
    hide_window, init_search_bar_window, save_app_config, save_key_filter_data, save_path_config,
    show_setting_window, update_search_bar_window,
};
use crate::program_manager::PROGRAM_MANAGER;
use crate::singleton::Singleton;
use crate::ui_controller::handle_focus_lost;
use config::{Height, RuntimeConfig, Width};
use rdev::{listen, Event, EventType, Key};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use tauri::async_runtime::spawn;
use tauri::Size;
use tauri::{webview::WebviewWindow, Emitter, Manager, PhysicalPosition, PhysicalSize};
use tauri_plugin_autostart::{AutoLaunchManager, MacosLauncher};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let windows: Arc<Vec<WebviewWindow>> =
                Arc::new(app.webview_windows().values().cloned().collect());

            let windows_clone = Arc::clone(&windows);
            let main_window = Arc::new(app.get_webview_window("main").unwrap());
            let app_handle = app.app_handle().clone();
            *GLOBAL_APP_HANDLE.lock().unwrap() = Some(app_handle.clone());
            init_setting_window(app_handle.clone());
            handle_auto_start();
            tauri::async_runtime::spawn(async move {
                start_key_listener(app_handle.clone()).expect("Failed to start key listener");
            });
            main_window.on_window_event(move |event| match event {
                tauri::WindowEvent::Focused(focused) => {
                    if !focused {
                        handle_focus_lost(&windows_clone);
                    }
                }
                _ => {}
            });

            let monitor = main_window.current_monitor().unwrap().unwrap();
            let size = monitor.size();

            let config_instance = RuntimeConfig::instance();
            let mut config = config_instance.lock().unwrap();
            config.set_sys_window_size(size.width as Width, size.height as Height);
            let scale_factor = main_window.scale_factor().unwrap_or(1.0);
            config.set_window_scale_factor(scale_factor);

            let position = config.get_window_render_origin();
            main_window
                .set_position(PhysicalPosition::new(position.0 as u32, position.1 as u32))
                .unwrap();
            let window_size = config.get_window_size();
            main_window
                .set_size(PhysicalSize::new(
                    window_size.0 as u32 + (20 as f64 * scale_factor) as u32,
                    window_size.1 as u32 + (20 as f64 * scale_factor) as u32,
                ))
                .unwrap();
            drop(config);
            update_app_setting();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            init_search_bar_window,
            handle_search_text,
            hide_window,
            show_setting_window,
            get_app_config,
            save_app_config,
            update_search_bar_window,
            save_path_config,
            get_path_config,
            get_key_filter_data,
            get_program_info,
            save_key_filter_data
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn handle_pressed(app_handle: tauri::AppHandle) {
    let main_window = Arc::new(app_handle.get_webview_window("main").unwrap());
    main_window.show().unwrap();
    main_window.set_focus().unwrap();
    main_window.emit("show_window", ()).unwrap();
}

fn start_key_listener(app_handle: tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let pressed_keys = Arc::new(Mutex::new(HashSet::new()));
    let pressed_keys_clone = Arc::clone(&pressed_keys);

    let callback = move |event: Event| {
        let mut keys = pressed_keys_clone.lock().unwrap();

        match event.event_type {
            EventType::KeyPress(key) => {
                keys.insert(key.clone());

                if keys.contains(&Key::Alt) && keys.contains(&Key::Space) {
                    handle_pressed(app_handle.clone());
                    keys.clear();
                }
            }
            EventType::KeyRelease(key) => {
                keys.remove(&key);
            }
            _ => {}
        }
    };

    if let Err(error) = listen(callback) {
        println!("监听器启动失败: {:?}", error);
    }

    Ok(())
}

fn init_setting_window(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let setting_window = Arc::new(
            tauri::WebviewWindowBuilder::new(
                &app,
                "setting_window",
                tauri::WebviewUrl::App("http://localhost:1420/setting_window".into()),
            )
            .title("设置")
            .visible(false)
            .build()
            .unwrap(),
        );
        let window_clone = Arc::clone(&setting_window);
        setting_window.on_window_event(move |event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // 阻止窗口关闭
                api.prevent_close();
                // 隐藏窗口
                window_clone.hide().unwrap();
                println!("隐藏设置窗口");
            }
        });
    });
}

/// 更新程序的状态
pub fn update_app_setting() {
    // 1. 重新更新程序索引的路径
    update_program_path();
    // 2. 判断要不要开机自启动
    handle_auto_start();
    // 3.判断要不要静默启动
    handle_silent_start();
}
/// 重新索引程序
pub fn update_program_path() {
    let instance = RuntimeConfig::instance();
    let runtime_config = instance.lock().unwrap();
    let mut program_manager = PROGRAM_MANAGER.lock().unwrap();
    program_manager.load_from_config(runtime_config.get_program_manager_config());
}

/// 处理自动开机的逻辑
pub fn handle_auto_start() {
    let mut instance = GLOBAL_APP_HANDLE.lock().unwrap();
    let app = instance.as_mut().unwrap();
    use tauri_plugin_autostart::MacosLauncher;
    use tauri_plugin_autostart::ManagerExt;

    app.plugin(tauri_plugin_autostart::init(
        MacosLauncher::LaunchAgent,
        None,
    ));

    // Get the autostart manager
    let autostart_manager = app.autolaunch();

    let instance = RuntimeConfig::instance();
    let runtime_config = instance.lock().unwrap();
    let is_auto_start = runtime_config.get_app_config().is_auto_start;
    if is_auto_start && !autostart_manager.is_enabled().unwrap() {
        autostart_manager.enable();
    }
    if !is_auto_start && autostart_manager.is_enabled().unwrap() {
        autostart_manager.disable();
    }
}

/// 处理静默启动
pub fn handle_silent_start() {
    let mut instance = GLOBAL_APP_HANDLE.lock().unwrap();
    let app = instance.as_mut().unwrap();
    let main_window = app.get_webview_window("main").unwrap();

    let instance = RuntimeConfig::instance();
    let runtime_config = instance.lock().unwrap();
    let app_config = runtime_config.get_app_config();
    if app_config.is_silent_start {
        main_window.hide();
    } else {
        main_window.show();
    }
}
