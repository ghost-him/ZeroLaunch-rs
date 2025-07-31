pub mod commands;
pub mod core;
pub mod error;
pub mod modules;
pub mod state;
pub mod tray;
pub mod utils;
pub mod window_effect;
pub mod window_position;
use crate::commands::config_file::*;
use crate::commands::debug::*;
use crate::commands::program_service::*;
use crate::commands::shortcut::*;
use crate::commands::ui_command::*;
use crate::commands::utils::*;
use crate::modules::config::config_manager::PartialRuntimeConfig;
use crate::modules::config::default::LOCAL_CONFIG_PATH;
use crate::modules::config::default::LOG_DIR;
use crate::modules::config::default::REMOTE_CONFIG_DEFAULT;
use crate::modules::config::{Height, Width};
use crate::modules::ui_controller::controller::get_window_render_origin;
use crate::modules::ui_controller::controller::get_window_size;
use crate::state::app_state::AppState;
use crate::tray::init_system_tray;
use crate::utils::defer::defer;
use crate::utils::ui_controller::handle_focus_lost;
use crate::utils::ui_controller::handle_pressed;
use backtrace::Backtrace;
use chrono::DateTime;
use chrono::Duration;
use chrono::Local;
use core::storage;
use core::storage::storage_manager::StorageManager;
use device_query::DeviceQuery;
use device_query::DeviceState;
use modules::config::app_config::PartialAppConfig;
use modules::config::config_manager::RuntimeConfig;
use modules::config::default::{APP_PIC_PATH, REMOTE_CONFIG_NAME};
use modules::config::load_local_config;
use modules::config::save_local_config;
use modules::config::ui_config::PartialUiConfig;
use modules::config::window_state::PartialWindowState;
use modules::program_manager::config::image_loader_config::RuntimeImageLoaderConfig;
use modules::program_manager::config::program_manager_config::RuntimeProgramConfig;
use modules::program_manager::{self, ProgramManager};
use modules::shortcut_manager::shortcut_manager::start_shortcut_manager;
use modules::shortcut_manager::shortcut_manager::update_shortcut_manager;
use modules::ui_controller::controller::recommend_footer_height;
use modules::ui_controller::controller::recommend_result_item_height;
use modules::ui_controller::controller::recommend_search_bar_height;
use modules::ui_controller::controller::recommend_window_width;
use std::fs::File;
use std::io::Write;
use std::panic;
use std::path::Path;
use std::sync::Arc;
use tauri::App;
use tauri::Emitter;
use tauri::LogicalSize;
use tauri::Manager;
use tauri::WebviewUrl;
use tauri_plugin_deep_link::DeepLinkExt;
use tracing::warn;
use tracing::Level;
use tracing::{debug, error, info};
use tracing_appender::rolling::RollingFileAppender;
use tracing_appender::rolling::Rotation;
use utils::notify::notify;
use utils::service_locator::ServiceLocator;
use window_effect::enable_window_effect;
use window_position::update_window_size_and_position;
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

        let backtrace = Backtrace::new();

        writeln!(file, "backtracing: {:?}", backtrace).expect("Could not write to panic log file");

        error!("Panic occurred: {}", message);
    }));

    cleanup_old_logs(&LOG_DIR.to_string(), 5);

    let com_init = unsafe { windows::Win32::System::Com::CoInitialize(None) };
    if com_init.is_err() {
        warn!("初始化com库失败：{:?}", com_init);
    }

    defer(move || unsafe {
        if com_init.is_ok() {
            windows::Win32::System::Com::CoUninitialize();
        }
    });

    let builder = tauri::Builder::default().plugin(tauri_plugin_shell::init());
    builder
        .plugin(tauri_plugin_single_instance::init(|_app, _argv, _cwd| {
            notify("zerolaunch-rs", "zerolaunch-rs已运行，不要重复运行");
        }))
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .manage(Arc::new(AppState::new()))
        .setup(|app| {
            tauri::async_runtime::block_on(async move {
                // 初始化程序的图标
                register_icon_path(app);
                // 初始化程序的配置系统
                init_app_state(app).await;
                // 初始化程序的系统托盘服务
                init_system_tray(app);
                // 初始化搜索栏
                init_search_bar_window(app);
                // 初始化设置窗口
                init_setting_window(app.app_handle().clone());
                // 初始化键盘监听器
                start_shortcut_manager(app);
                // 根据配置信息更新整个程序
                update_app_setting().await;

                app.deep_link().register_all().unwrap();
                app.deep_link().on_open_url(|event| {
                    tauri::async_runtime::spawn(async move {
                        let state = ServiceLocator::get_state();
                        let waiting_hashmap = state.get_waiting_hashmap();
                        for url in event.urls() {
                            let domain = url.domain().unwrap().to_string();
                            let mut pairs = Vec::new();
                            url.query_pairs().into_iter().for_each(|(key, value)| {
                                pairs.push((key.to_string(), value.to_string()));
                            });
                            waiting_hashmap.insert(domain, pairs).await;
                        }
                    });
                });
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            command_save_remote_config,
            load_program_icon,
            get_program_count,
            launch_program,
            get_program_info,
            refresh_program,
            handle_search_text,
            update_search_bar_window,
            get_background_picture,
            get_remote_config_dir,
            select_background_picture,
            hide_window,
            show_setting_window,
            command_load_remote_config,
            get_dominant_color,
            command_get_latest_release_version,
            test_search_algorithm,
            test_search_algorithm_time,
            test_index_app_time,
            get_search_keys,
            command_get_default_remote_data_dir_path,
            command_load_local_config,
            command_save_local_config,
            command_check_validation,
            open_target_folder,
            command_unregister_all_shortcut,
            command_register_all_shortcut,
            command_change_tray_icon,
            command_open_icon_cache_dir,
            command_get_system_fonts,
            command_get_path_info,
            command_get_latest_launch_propgram //command_get_onedrive_refresh_token
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// 初始化的流程-> 初始化程序的状态
async fn init_app_state(app: &mut App) {
    // 维护程序状态
    let state = app.state::<Arc<AppState>>();
    ServiceLocator::init((*state).clone());
    let state = ServiceLocator::get_state();
    // 维护app_handle
    state.set_main_handle(Arc::new(app.app_handle().clone()));

    // 在初始化时传入一个函数，这个函数会初始化一个新版本的提示

    let create_and_show_welcome_page = move || {
        info!("第一次启动程序或者更新程序，创建欢迎页面");
        // 创建欢迎页面
        let welcome = Arc::new(
            tauri::WebviewWindowBuilder::new(app, "welcome", WebviewUrl::App("/welcome".into()))
                .title("欢迎下载ZeroLaunch-rs! 此页面只会出现一次，用于提供基础的使用说明╰(*°▽°*)╯")
                .visible(true)
                .drag_and_drop(false)
                .build()
                .unwrap(),
        );
        welcome.set_size(LogicalSize::new(950, 500)).unwrap();
    };
    create_and_show_welcome_page();
    let storage_manager = StorageManager::new(create_and_show_welcome_page).await;
    let remote_config_data = {
        if let Some(data) = storage_manager
            .download_file_str(REMOTE_CONFIG_NAME.to_string())
            .await
        {
            data
        } else {
            storage_manager
                .upload_file_str(
                    REMOTE_CONFIG_NAME.to_string(),
                    REMOTE_CONFIG_DEFAULT.clone(),
                )
                .await;
            REMOTE_CONFIG_DEFAULT.clone()
        }
    };

    let partial_config = load_local_config(&remote_config_data);

    let runtime_config = RuntimeConfig::new();
    runtime_config.update(partial_config);

    // 维护程序的配置信息
    state.set_runtime_config(Arc::new(runtime_config));
    // 维护程序管理器
    let runtime_program_config = RuntimeProgramConfig {
        image_loader_config: RuntimeImageLoaderConfig {
            default_app_icon_path: APP_PIC_PATH.get("tips").unwrap().value().clone(),
            default_web_icon_path: APP_PIC_PATH.get("web_pages").unwrap().value().clone(),
        },
    };

    let program_manager = ProgramManager::new(runtime_program_config);
    state.set_program_manager(Arc::new(program_manager));

    // 维护文件管理器
    state.set_storage_manager(Arc::new(storage_manager));
    // 使用ServiceLocator保存一份
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
        sys_window_locate_height: Some(0),
        sys_window_locate_width: Some(0),
    });

    update_window_size_and_position();
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
                // 获取当前鼠标位置
                let device_state = DeviceState::new();
                let mouse_state = device_state.get_mouse();
                let position = mouse_state.coords;
                // 获取窗口位置和大小
                if let Ok(window_position) = windows_clone.inner_position() {
                    if let Ok(window_size) = windows_clone.inner_size() {
                        // 检查鼠标是否在窗口内
                        let in_window = position.0 >= window_position.x
                            && position.0 <= window_position.x + window_size.width as i32
                            && position.1 >= window_position.y
                            && position.1 <= window_position.y + window_size.height as i32;
                        // 只有当鼠标不在窗口内时才隐藏
                        if !in_window {
                            handle_focus_lost(windows_clone.clone());
                        }
                    }
                }
            }
        }
    });
    // 初始化完成后就隐藏
    handle_focus_lost(main_window.clone());
}

///注册图标的路径
fn register_icon_path(app: &mut App) {
    let path_resolver = app.path();
    let resource_icons_dir = path_resolver
        .resource_dir()
        .expect("无法获取资源目录")
        .join("icons");

    // 定义图标的键名和对应的文件名
    // (键名, 文件名)
    let icons_to_register = [
        ("tray_icon", "32x32.png"),
        ("tray_icon_white", "32x32-white.png"),
        ("web_pages", "web_pages.png"),
        ("tips", "tips.png"),
        ("terminal", "terminal.png"),
        ("settings", "settings.ico"),
        ("refresh", "refresh.ico"),
        ("register", "register.ico"),
        ("game", "game.ico"),
        ("exit", "exit.ico"),
    ];

    for (key_name, file_name) in icons_to_register.iter() {
        let icon_path = resource_icons_dir.join(file_name);
        match icon_path.to_str() {
            Some(path_str) => {
                APP_PIC_PATH.insert(key_name.to_string(), path_str.to_string());
            }
            None => {
                // 处理路径无法转换为 UTF-8 字符串的情况
                // 在这个特定场景下，图标文件名通常是 ASCII/UTF-8，所以 .unwrap() 可能也能接受
                // 但更健壮的做法是处理 None 的情况
                warn!(
                    "警告: 路径 {:?} 无法转换为有效的UTF-8字符串，跳过图标 '{}'",
                    icon_path, key_name
                );
            }
        }
    }
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
            .drag_and_drop(false)
            .build()
            .unwrap(),
        );
        setting_window
            .set_size(LogicalSize::new(950, 500))
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

/// 更新程序的状态
async fn update_app_setting() {
    let state = ServiceLocator::get_state();
    // 如果当前可见，则忽略更新
    if state.get_search_bar_visible() {
        return;
    }

    let runtime_config = state.get_runtime_config().unwrap();

    // 1. 重新更新程序索引的路径
    let program_manager = state.get_program_manager().unwrap();
    program_manager
        .load_from_config(runtime_config.get_program_manager_config())
        .await;

    // 2. 判断要不要开机自启动
    if let Err(e) = handle_auto_start() {
        // 可以添加错误处理逻辑
        eprintln!("自启动设置失败: {:?}", e);
    }

    // 3.判断要不要静默启动
    handle_silent_start();

    // 4.判断要不要更新当前的窗口大小
    update_window_size_and_position();

    // 5.更新当前的窗口效果
    enable_window_effect();

    // 6.更新快捷键的绑定
    update_shortcut_manager();

    // 获取主窗口句柄
    if let Ok(handle) = state.get_main_handle() {
        // 发送事件
        if let Err(e) = handle.emit("update_search_bar_window", "") {
            eprintln!("发送窗口更新事件失败: {:?}", e);
        }
    } else {
        println!("无法找到目标窗口");
    }

    let mins = runtime_config.get_app_config().get_auto_refresh_time() as u64;
    // 取消当前的定时器
    if let Some(guard) = state.take_timer_guard() {
        drop(guard); // 取消定时器
    }
    // 创建新定时器
    let new_interval = Duration::seconds((mins * 60) as i64);
    let timer = state.get_timer();
    // 使用 spawn_local 来处理异步定时任务
    let guard_value = timer.schedule_repeating(new_interval, move || {
        // 创建一个新的任务来执行异步函数
        tauri::async_runtime::block_on(async {
            update_app_setting().await;
        });
    });
    state.set_timer_guard(guard_value);
}

/// 保存程序的配置信息
/// 1. 将需要保存的东西保到配置信息中
/// 2. 保存动态数据
/// 3. 保存到文件中
/// 4. 重新读取文件并更新配置信息

pub async fn save_config_to_file(is_update_app: bool) {
    let state = ServiceLocator::get_state();
    let runtime_config = state.get_runtime_config().unwrap();
    let runtime_data = state
        .get_program_manager()
        .unwrap()
        .get_runtime_data()
        .await;
    let window = state
        .get_main_handle()
        .unwrap()
        .get_webview_window("main")
        .unwrap()
        .inner_position()
        .unwrap();

    let mut partial_app_config = PartialAppConfig::default();
    partial_app_config.window_position = Some((window.x, window.y));

    runtime_config.update(PartialRuntimeConfig {
        app_config: Some(partial_app_config),
        ui_config: None,
        shortcut_config: None,
        program_manager_config: Some(runtime_data),
        window_state: None,
    });
    let remote_config = runtime_config.to_partial();

    let data_str = save_local_config(remote_config);

    let storage_manager = state.get_storage_manager().unwrap();

    storage_manager
        .upload_file_str(REMOTE_CONFIG_NAME.to_string(), data_str)
        .await;

    if is_update_app {
        update_app_setting().await;
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
        let runtime_config = state.get_runtime_config().unwrap();
        let app_config = runtime_config.get_app_config();
        if !app_config.get_is_silent_start() {
            notify("ZeroLaunch-rs", "ZeroLaunch-rs已成功启动！");
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
