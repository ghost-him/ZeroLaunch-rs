pub mod config;
pub mod program_manager;
pub mod singleton;
pub mod ui_controller;
pub mod utils;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use crate::program_manager::PROGRAM_MANAGER;
use crate::singleton::Singleton;
use crate::ui_controller::handle_focus_lost;
use crate::utils::{
    get_item_size, get_window_scale_factor, get_window_size, handle_search_text, hide_window,
    show_setting_window,
};
use config::{Height, RuntimeConfig, Width};
use rdev::{listen, Event, EventType, Key};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use tauri::async_runtime::spawn;
use tauri::{webview::WebviewWindow, Emitter, Manager, PhysicalPosition, PhysicalSize};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_window_size,
            get_item_size,
            get_window_scale_factor,
            handle_search_text,
            hide_window,
            show_setting_window,
        ])
        .setup(|app| {
            let windows: Arc<Vec<WebviewWindow>> =
                Arc::new(app.webview_windows().values().cloned().collect());

            let windows_clone = Arc::clone(&windows);
            let main_window = Arc::new(app.get_webview_window("main").unwrap());
            let app_handle = app.handle().clone();
            init_setting_window(app_handle.clone());
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

            let mut program_manager = PROGRAM_MANAGER.lock().unwrap();
            program_manager.load_from_config(config.get_program_manager_config());
            Ok(())
        })
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
