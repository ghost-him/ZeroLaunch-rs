pub mod config;
pub mod defer;
pub mod interface;
pub mod program_manager;
pub mod singleton;
pub mod ui_controller;
pub mod utils;
use std::panic;
// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use crate::config::CONFIG_PATH;
use crate::config::GLOBAL_APP_HANDLE;
use crate::config::LOG_DIR;
use crate::interface::{
    get_app_config, get_key_filter_data, get_path_config, get_program_info, handle_search_text,
    hide_window, init_search_bar_window, launch_program, load_program_icon, save_app_config,
    save_key_filter_data, save_path_config, show_setting_window, update_search_bar_window,
};
use crate::program_manager::PROGRAM_MANAGER;
use crate::singleton::Singleton;
use crate::ui_controller::handle_focus_lost;
use config::{Height, RuntimeConfig, Width};
use rdev::ListenError;
use rdev::{listen, Event, EventType, Key};
use single_instance::SingleInstance;
use std::collections::HashSet;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Once;
use std::sync::{Arc, Mutex};
use tauri::image::Image;
use tauri::menu::{MenuBuilder, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::App;
use tauri::WebviewUrl;
use tauri::{webview::WebviewWindow, Emitter, Manager, PhysicalPosition, PhysicalSize};
use tracing::Level;
use tracing::{debug, error, info, trace, warn};
use tracing_appender::rolling::RollingFileAppender;
use tracing_appender::rolling::Rotation;
use tracing_subscriber::{fmt, EnvFilter};
use windows::Win32::Foundation::{BOOL, LPARAM, TRUE};
use windows::Win32::System::Console::{
    SetConsoleCtrlHandler, CTRL_CLOSE_EVENT, CTRL_SHUTDOWN_EVENT,
};
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 创建一个按日期滚动的日志文件，例如每天一个新文件
    let file_appender: RollingFileAppender =
        RollingFileAppender::new(Rotation::DAILY, LOG_DIR.clone(), "info.log");

    // 创建一个非阻塞的日志写入器
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // 配置订阅者
    let subscriber = tracing_subscriber::fmt()
        .with_writer(non_blocking) // 设置日志输出到文件
        .with_max_level(Level::INFO) // 设置日志级别
        .with_ansi(false)
        .finish();

    // 设置全局默认的订阅者
    tracing::subscriber::set_global_default(subscriber).expect("设置全局默认订阅者失败");

    // 设置 panic hook
    panic::set_hook(Box::new(|panic_info| {
        let location = panic_info.location().unwrap();
        let message = match panic_info.payload().downcast_ref::<&str>() {
            Some(s) => *s,
            None => "Unknown panic message",
        };

        let log_dir = LOG_DIR.clone();
        let panic_file_path = Path::new(&log_dir)
            .join("panic.log")
            .to_str()
            .unwrap()
            .to_string();
        let mut file = File::create(panic_file_path).expect("Could not create panic log file");
        writeln!(
            file,
            "Panic occurred in file '{}' at line {}: {}",
            location.file(),
            location.line(),
            message
        )
        .expect("Could not write to panic log file");

        error!("Panic occurred: {}", message);
    }));

    let instance = SingleInstance::new("ZeroLaunch-rs").unwrap();
    if !instance.is_single() {
        error!("当前已经有实例在运行了");
        std::process::exit(1);
    }
    tauri::Builder::default()
        .setup(|app| {
            let windows: Arc<Vec<WebviewWindow>> =
                Arc::new(app.webview_windows().values().cloned().collect());
            init_system_tray(app);
            let windows_clone = Arc::clone(&windows);
            let main_window = Arc::new(app.get_webview_window("main").unwrap());
            let app_handle = app.app_handle().clone();
            *GLOBAL_APP_HANDLE.lock().unwrap() = Some(app_handle.clone());
            init_setting_window(app_handle.clone());
            handle_auto_start();
            tauri::async_runtime::spawn(async move {
                start_key_listener(app_handle.clone()).expect("Failed to start key listener");
            });
            main_window.on_window_event(move |event| {
                if let tauri::WindowEvent::Focused(focused) = event {
                    if !focused {
                        handle_focus_lost(&windows_clone);
                    }
                }
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
                    window_size.0 as u32 + (20_f64 * scale_factor) as u32,
                    window_size.1 as u32 + (20_f64 * scale_factor) as u32,
                ))
                .unwrap();
            drop(config);
            update_app_setting();
            // PROGRAM_MANAGER.lock().unwrap().test_search_algorithm("");
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
            save_key_filter_data,
            launch_program,
            load_program_icon,
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

fn start_key_listener(app_handle: tauri::AppHandle) -> Result<(), ListenError> {
    let pressed_keys = Arc::new(Mutex::new(HashSet::new()));
    let pressed_keys_clone: Arc<Mutex<HashSet<Key>>> = Arc::clone(&pressed_keys);

    let callback = move |event: Event| {
        let mut keys = pressed_keys_clone.lock().unwrap();

        match event.event_type {
            EventType::KeyPress(key) => {
                keys.insert(key);

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
        error!("监听器启动失败: {:?}", error);
        return Err(error);
    }

    Ok(())
}

fn init_setting_window(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let setting_window = Arc::new(
            tauri::WebviewWindowBuilder::new(
                &app,
                "setting_window",
                WebviewUrl::App("/setting_window".into()),
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
                debug!("隐藏设置窗口");
            }
        });
    });
}

enum MenuEventId {
    ShowSettingWindow,
    ExitProgram,
    UpdateAppSetting,
    Unknown(String),
}

// 从事件 ID 转换为枚举
impl From<&str> for MenuEventId {
    fn from(id: &str) -> Self {
        match id {
            "show_setting_window" => MenuEventId::ShowSettingWindow,
            "exit_program" => MenuEventId::ExitProgram,
            "update_app_setting" => MenuEventId::UpdateAppSetting,
            _ => MenuEventId::Unknown(id.to_string()),
        }
    }
}

/// 创建一个右键菜单
fn init_system_tray(app: &mut App) {
    let handle = app.handle();
    let menu = MenuBuilder::new(app)
        .item(
            &MenuItem::with_id(
                handle,
                "show_setting_window",
                "打开设置界面",
                true,
                None::<&str>,
            )
            .unwrap(),
        )
        .item(&MenuItem::with_id(handle, "exit_program", "退出程序", true, None::<&str>).unwrap())
        .build()
        .unwrap();
    let path_resolver = app.path();
    let resource = path_resolver.resource_dir().expect("无法获取资源目录");
    let icon_path: PathBuf = resource.join("icons").join("32x32.png");
    info!("icon path: {:?}", icon_path);
    let tray_icon = TrayIconBuilder::new()
        .menu(&menu)
        .icon(Image::from_path(icon_path).unwrap())
        .tooltip("ZeroLaunch-rs v0.1.0")
        .build(handle)
        .unwrap();
    tray_icon.on_menu_event(|app_handle, event| {
        let event_id = MenuEventId::from(event.id().as_ref());
        match event_id {
            MenuEventId::ShowSettingWindow => {
                if let Err(e) = show_setting_window(app_handle.clone()) {
                    warn!("Failed to show setting window: {:?}", e);
                }
            }
            MenuEventId::ExitProgram => {
                save_config_to_file(false);
                app_handle.exit(0);
            }
            MenuEventId::UpdateAppSetting => {
                update_app_setting();
            }
            MenuEventId::Unknown(id) => {
                warn!("Unknown menu event: {}", id);
            }
        }
        debug!("Menu ID: {}", event.id().0);
    });
}

/// 更新程序的状态
fn update_app_setting() {
    // 1. 重新更新程序索引的路径
    update_program_path();
    // 2. 判断要不要开机自启动
    handle_auto_start();
    // 3.判断要不要静默启动
    handle_silent_start();
}
/// 保存程序的配置信息
/// 1. 将需要保存的东西保到配置信息中
/// 2. 保存到文件中
/// 3. 重新读取文件并更新配置信息

pub fn save_config_to_file(is_update_app: bool) {
    let mut manager = PROGRAM_MANAGER.lock().unwrap();
    let config = manager.get_launcher_config();
    drop(manager);
    {
        let instance = RuntimeConfig::instance();
        let mut runtime_config = instance.lock().unwrap();
        runtime_config.save_program_launcher_config(&config);
        let config_content: String = runtime_config.save_config();
        std::fs::write(&*CONFIG_PATH, config_content).unwrap();
    }
    if is_update_app {
        update_app_setting();
    }
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
    ))
    .unwrap();

    // Get the autostart manager
    let autostart_manager = app.autolaunch();

    let instance = RuntimeConfig::instance();
    let runtime_config = instance.lock().unwrap();
    let is_auto_start = runtime_config.get_app_config().is_auto_start;
    if is_auto_start && !autostart_manager.is_enabled().unwrap() {
        let _ = autostart_manager.enable();
    }
    if !is_auto_start && autostart_manager.is_enabled().unwrap() {
        let _ = autostart_manager.disable();
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
        let _ = main_window.hide();
    } else {
        let _ = main_window.show();
    }
}
