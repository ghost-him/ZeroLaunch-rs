pub mod commands;
pub mod core;
pub mod error;
pub mod modules;
pub mod state;
pub mod utils;
use crate::commands::debug::*;
use crate::commands::file::*;
use crate::commands::program_service::*;
use crate::commands::ui_command::*;
use crate::commands::utils::*;
use crate::modules::config::config_manager::PartialConfig;
use crate::modules::config::default::LOCAL_CONFIG_PATH;
use crate::modules::config::default::LOG_DIR;
use crate::modules::config::local_config::LocalConfig;
use crate::modules::config::{Height, Width};
use crate::modules::storage::utils::read_or_create_str;
use crate::modules::ui_controller::controller::get_window_render_origin;
use crate::modules::ui_controller::controller::get_window_size;
use crate::state::app_state::AppState;
use crate::utils::get_remote_config_path;
use crate::utils::ui_controller::handle_focus_lost;
use crate::utils::ui_controller::handle_pressed;
use chrono::DateTime;
use chrono::Duration;
use chrono::Local;
use modules::config::config_manager::RuntimeConfig;
use modules::config::default::APP_VERSION;
use modules::config::default::{APP_PIC_PATH, REMOTE_CONFIG_NAME};
use modules::config::save_remote_config;
use modules::config::window_state::PartialWindowState;
use modules::program_manager::{self, ProgramManager};
use rdev::{listen, Event, EventType, Key};
use single_instance::SingleInstance;
use std::collections::HashSet;
use std::fs::File;
use std::io::Write;
use std::panic;
use std::path::Path;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::image::Image;
use tauri::menu::{MenuBuilder, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::tray::TrayIconEvent;
use tauri::App;
use tauri::Emitter;
use tauri::WebviewUrl;
use tauri::{Manager, PhysicalPosition, PhysicalSize};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_dialog::MessageDialogKind;
use tauri_plugin_notification::NotificationExt;
use tracing::Level;
use tracing::{debug, error, info, warn};
use tracing_appender::rolling::RollingFileAppender;
use tracing_appender::rolling::Rotation;
use utils::service_locator::ServiceLocator;
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
        .with_max_level(Level::DEBUG) // 设置日志级别
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

    cleanup_old_logs(&LOG_DIR.to_string(), 5);

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .manage(Arc::new(AppState::new()))
        .setup(|app| {
            let instance = SingleInstance::new("ZeroLaunch-rs").unwrap();
            if !instance.is_single() {
                error!("当前已经有实例在运行了");
                app.dialog()
                    .message("当前的程序已经在运行了")
                    .kind(MessageDialogKind::Error)
                    .title("注意")
                    .blocking_show();

                std::process::exit(1);
            }
            // 初始化程序的图标
            register_icon_path(app);
            // 初始化程序的配置系统
            init_app_state(app);
            // 初始化程序的系统托盘服务
            init_system_tray(app);
            // 初始化搜索栏
            init_search_bar_window(app);
            // 初始化设置窗口
            init_setting_window(app.app_handle().clone());
            // 初始化键盘监听器
            start_key_listener(app);
            // 根据配置信息更新整个程序
            update_app_setting();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            save_config,
            load_program_icon,
            get_program_count,
            launch_program,
            get_program_info,
            refresh_program,
            handle_search_text,
            initialize_search_window,
            update_search_bar_window,
            get_background_picture,
            change_remote_config_dir,
            get_remote_config_dir,
            select_background_picture,
            hide_window,
            show_setting_window,
            load_config,
            get_dominant_color,
            command_get_latest_release_version,
            test_search_algorithm,
            test_search_algorithm_time,
            test_index_app_time,
            get_search_keys,
            command_get_default_remote_data_dir_path
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// 初始化的流程-> 初始化程序的状态
fn init_app_state(app: &mut App) {
    // 读取本地的配置文件信息，获得远程配置文件的地址
    let local_config = LocalConfig::default();
    // 先读一下配置文件的地址
    let local_config_data = read_or_create_str(
        &LOCAL_CONFIG_PATH,
        Some(serde_json::to_string(&local_config).unwrap()),
    )
    .expect("无法读取");
    let local_config: LocalConfig = serde_json::from_str(&local_config_data).unwrap();
    let remote_config_dir_path = local_config.remote_config_path;
    let remote_config_path = Path::new(&remote_config_dir_path)
        .join(REMOTE_CONFIG_NAME)
        .to_str()
        .unwrap()
        .to_string();
    let runtime_config = RuntimeConfig::new(remote_config_path.clone());
    runtime_config.load_from_remote_config_path(None);
    // 维护程序状态
    let state = app.state::<Arc<AppState>>();
    // 设置远程配置存在的位置
    state.set_remote_config_dir_path(remote_config_dir_path);
    // 维护程序的配置信息
    state.set_runtime_config(Arc::new(runtime_config));
    // 维护程序管理器
    let program_manager = ProgramManager::new(APP_PIC_PATH.get("tips").unwrap().value().clone());
    state.set_program_manager(Arc::new(program_manager));
    // 维护app_handle
    state.set_main_handle(Arc::new(app.app_handle().clone()));
    // 使用ServiceLocator保存一份
    ServiceLocator::init((*state).clone());
}

/// 初始化搜索界面的窗口设置
fn init_search_bar_window(app: &mut App) {
    let main_window = Arc::new(app.get_webview_window("main").unwrap());
    // 设置tauri窗口的大小等参数
    let monitor = main_window.current_monitor().unwrap().unwrap();
    // 获得了当前窗口的物理大小
    let size = monitor.size();
    let scale_factor = main_window.scale_factor().unwrap_or(1.0);
    let state = app.state::<Arc<AppState>>();
    let config = state.get_runtime_config().unwrap();

    config.get_window_state().update(PartialWindowState {
        sys_window_scale_factor: Some(scale_factor),
        sys_window_width: Some(size.width as Width),
        sys_window_height: Some(size.height as Height),
    });

    let position = get_window_render_origin();
    main_window
        .set_position(PhysicalPosition::new(position.0 as u32, position.1 as u32))
        .unwrap();
    let window_size = get_window_size();
    main_window
        .set_size(PhysicalSize::new(
            window_size.0 as u32,
            window_size.1 as u32 + (20_f64 * scale_factor) as u32,
        ))
        .unwrap();
    // 设置当窗口被关闭时，忽略
    let windows_clone = main_window.clone();
    main_window.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            // 阻止窗口关闭
            api.prevent_close();
            // 隐藏窗口
            handle_focus_lost(windows_clone.clone());
            debug!("隐藏设置窗口");
        }
        if let tauri::WindowEvent::Focused(focused) = event {
            if !focused {
                handle_focus_lost(windows_clone.clone());
            }
        }
    });
}

///注册图标的路径
fn register_icon_path(app: &mut App) {
    let path_resolver = app.path();
    let resource = path_resolver
        .resource_dir()
        .expect("无法获取资源目录")
        .join("icons");

    let icon_path: PathBuf = resource.join("32x32.png");
    APP_PIC_PATH.insert(
        "tray_icon".to_string(),
        icon_path.to_str().unwrap().to_string(),
    );

    let web_icon = resource.join("web_pages.png");
    APP_PIC_PATH.insert(
        "web_page".to_string(),
        web_icon.to_str().unwrap().to_string(),
    );
    let tips_icon = resource.join("tips.png");
    APP_PIC_PATH.insert("tips".to_string(), tips_icon.to_str().unwrap().to_string());
    let terminal = resource.join("terminal.png");
    APP_PIC_PATH.insert(
        "terminal".to_string(),
        terminal.to_str().unwrap().to_string(),
    );
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
        setting_window
            .set_size(PhysicalSize::new(1300, 800))
            .unwrap();
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
    let t = APP_PIC_PATH.get("tray_icon").unwrap();
    let icon_path = t.value();
    info!("icon path: {:?}", icon_path);
    let tray_icon = TrayIconBuilder::new()
        .menu(&menu)
        .icon(Image::from_path(icon_path).unwrap())
        .tooltip(format!("ZeroLaunch-rs v{}", APP_VERSION.clone()))
        .show_menu_on_left_click(false)
        .build(handle)
        .unwrap();
    tray_icon.on_menu_event(|app_handle, event| {
        let event_id = MenuEventId::from(event.id().as_ref());
        match event_id {
            MenuEventId::ShowSettingWindow => {
                if let Err(e) = show_setting_window() {
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

    app.on_tray_icon_event(|app_handle, event| match event {
        TrayIconEvent::DoubleClick { .. } => {
            handle_pressed(&app_handle);
        }
        _ => {}
    });
}

/// 更新程序的状态
fn update_app_setting() {
    let state = ServiceLocator::get_state();
    // 如果当前可见，则忽略更新
    if state.get_search_bar_visible() {
        return;
    }
    let runtime_config = state.get_runtime_config().unwrap();
    // 1. 重新更新程序索引的路径
    let program_manager = state.get_program_manager().unwrap();
    program_manager.load_from_config(runtime_config.get_program_manager_config());

    // 2. 判断要不要开机自启动
    handle_auto_start().unwrap();

    // 3.判断要不要静默启动
    handle_silent_start();

    let mins = runtime_config.get_app_config().get_auto_refresh_time() as u64;

    // 取消当前的定时器
    if let Some(guard) = state.take_timer_guard() {
        drop(guard); // 取消定时器
    }

    // 创建新定时器
    let new_interval = Duration::seconds((mins * 60) as i64);
    let timer = state.get_timer();
    let guard_value = timer.schedule_repeating(new_interval, move || {
        update_app_setting();
    });
    state.set_timer_guard(guard_value);

    // 获取主窗口句柄
    let handle = state.get_main_handle().unwrap();

    // 发送事件
    handle.emit("update_search_bar_window", "").unwrap();
}

/// 保存程序的配置信息
/// 1. 将需要保存的东西保到配置信息中
/// 2. 保存动态数据
/// 3. 保存到文件中
/// 4. 重新读取文件并更新配置信息

pub fn save_config_to_file(is_update_app: bool) {
    let state = ServiceLocator::get_state();

    let runtime_config = state.get_runtime_config().unwrap();
    let runtime_data = state.get_program_manager().unwrap().get_runtime_data();

    runtime_config.update(PartialConfig {
        app_config: None,
        ui_config: None,
        program_manager_config: Some(runtime_data),
        window_state: None,
    });
    let remote_config = runtime_config.to_partial();

    let data_str = save_remote_config(remote_config);
    let config_path_str = get_remote_config_path();
    std::fs::write(config_path_str, data_str).unwrap();

    if is_update_app {
        update_app_setting();
    }
}

/// 处理自动开机的逻辑
pub fn handle_auto_start() -> Result<(), Box<dyn std::error::Error>> {
    let state: Arc<AppState> = ServiceLocator::get_state();

    // 处理主窗口句柄
    let app_handle = state.get_main_handle().unwrap();

    // 初始化自动启动插件
    {
        use tauri_plugin_autostart::MacosLauncher;
        app_handle.plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))?;
    }
    use tauri_plugin_autostart::ManagerExt;
    // 获取自动启动管理器
    let autostart_manager = app_handle.autolaunch();

    // 获取运行时配置
    let is_auto_start = {
        let config = state.get_runtime_config().unwrap();
        config.get_app_config().get_is_auto_start()
    };

    // 根据配置强制更新自动启动状态和路径
    if is_auto_start {
        // 无论当前是否启用，启用自动启动以确保路径正确
        autostart_manager.enable()?;
    } else {
        // 如果已启用则禁用
        if autostart_manager.is_enabled()? {
            autostart_manager.disable()?;
        }
    }

    Ok(())
}

/// 处理静默启动
pub fn handle_silent_start() {
    use std::sync::Once;

    static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        let state: Arc<AppState> = ServiceLocator::get_state();
        let app_handle = state.get_main_handle().unwrap();
        let runtime_config = state.get_runtime_config().unwrap();
        let app_config = runtime_config.get_app_config();
        if !app_config.get_is_silent_start() {
            app_handle
                .notification()
                .builder()
                .title("ZeroLaunch-rs")
                .body("ZeroLaunch-rs已成功启动！")
                .show()
                .unwrap();
        }
    });
}

// 函数用于删除超过一星期的日志文件
fn cleanup_old_logs(log_dir: &str, retention_days: i64) {
    // 获取当前时间
    let now: DateTime<Local> = Local::now();

    // 读取日志目录中的所有文件
    let entries = match std::fs::read_dir(log_dir) {
        Ok(entries) => entries,
        Err(e) => {
            error!("无法读取日志目录 '{}': {}", log_dir, e);
            return;
        }
    };

    for entry in entries {
        if let Ok(entry) = entry {
            let path = entry.path();
            if path.is_file() {
                // 获取文件的元数据
                if let Ok(metadata) = std::fs::metadata(&path) {
                    // 获取文件的修改时间
                    if let Ok(modified) = metadata.modified() {
                        // 将 SystemTime 转换为 DateTime
                        let modified_datetime: DateTime<Local> = modified.into();
                        // 计算文件的年龄
                        let age = now.signed_duration_since(modified_datetime);
                        if age.num_days() > retention_days {
                            // 删除文件
                            if let Err(e) = std::fs::remove_file(&path) {
                                error!("无法删除旧日志文件 '{:?}': {}", path, e);
                            } else {
                                info!("已删除旧日志文件: {:?}", path);
                            }
                        }
                    }
                }
            }
        }
    }
}

fn start_key_listener(app: &mut tauri::App) {
    use tauri_plugin_global_shortcut::{
        Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState,
    };

    let alt_space_shortcut = Shortcut::new(Some(Modifiers::ALT), Code::Space);
    app.handle()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |_app, shortcut, event| {
                    if shortcut == &alt_space_shortcut {
                        match event.state() {
                            ShortcutState::Pressed => {}
                            ShortcutState::Released => {}
                        }
                    }
                })
                .build(),
        )
        .unwrap();
    if let Err(e) = app.global_shortcut().register(alt_space_shortcut) {
        app.handle()
            .notification()
            .builder()
            .title("ZeroLaunch-rs")
            .body("按键 Alt + Space 绑定失败，程序将退出")
            .show()
            .unwrap();
        error!("按键 Alt + Space 绑定失败: {:?}", e);
        app.cleanup_before_exit();
        std::process::exit(1);
    }
    let app_handle = app.handle().clone();
    tauri::async_runtime::spawn(async move {
        let pressed_keys = Arc::new(Mutex::new(HashSet::new()));
        let pressed_keys_clone: Arc<Mutex<HashSet<Key>>> = Arc::clone(&pressed_keys);

        let callback = move |event: Event| {
            let mut keys = pressed_keys_clone.lock().unwrap();

            match event.event_type {
                EventType::KeyPress(key) => {
                    keys.insert(key);

                    if keys.contains(&Key::Alt) && keys.contains(&Key::Space) {
                        handle_pressed(&app_handle);
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
        }
    });
}
