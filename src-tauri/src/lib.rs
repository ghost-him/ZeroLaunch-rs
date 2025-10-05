pub mod commands;
pub mod core;
pub mod error;
pub mod logging;
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
#[cfg(feature = "ai")]
use crate::core::ai::model_manager::ModelManager;
use crate::error::{OptionExt, ResultExt};
use crate::logging::{init_logging, log_application_start, update_log_level};
use crate::modules::config::config_manager::PartialRuntimeConfig;
use crate::modules::config::default::LOCAL_CONFIG_PATH;
use crate::modules::config::default::REMOTE_CONFIG_DEFAULT;
use crate::modules::config::default::SEMANTIC_DESCRIPTION_FILE_NAME;
use crate::modules::config::{Height, Width};
use crate::modules::ui_controller::controller::get_window_render_origin;
use crate::state::app_state::AppState;
use crate::tray::init_system_tray;
use crate::utils::ui_controller::handle_focus_lost;
use crate::utils::ui_controller::handle_pressed;
use crate::window_position::update_window_size_and_position;
use chrono::Duration;
use core::storage;
use core::storage::storage_manager::StorageManager;
use device_query::DeviceQuery;
use device_query::DeviceState;
use modules::config::app_config::PartialAppConfig;
use modules::config::config_manager::RuntimeConfig;
use modules::config::default::{
    APP_PIC_PATH, REMOTE_CONFIG_NAME, SEMANTIC_EMBEDDING_CACHE_FILE_NAME,
};
use modules::config::load_string_to_runtime_config_;
use modules::config::save_runtime_config_to_string;
use modules::config::ui_config::PartialUiConfig;
use modules::config::window_state::PartialWindowState;
use modules::program_manager::config::image_loader_config::RuntimeImageLoaderConfig;
use modules::program_manager::config::program_manager_config::RuntimeProgramConfig;
use modules::program_manager::semantic_backend;
use modules::program_manager::{self, ProgramManager};
use modules::shortcut_manager::start_shortcut_manager;
use modules::shortcut_manager::update_shortcut_manager;
use modules::ui_controller::controller::recommend_footer_height;
use modules::ui_controller::controller::recommend_result_item_height;
use modules::ui_controller::controller::recommend_search_bar_height;
use modules::ui_controller::controller::recommend_window_width;
use std::path::Path;
use std::sync::Arc;
use tauri::App;
use tauri::Emitter;
use tauri::LogicalSize;
use tauri::Manager;
use tauri::WebviewUrl;
use tauri_plugin_deep_link::DeepLinkExt;
use tracing::{debug, error, info, warn};
use utils::notify::notify;
use utils::service_locator::ServiceLocator;
use window_effect::enable_window_effect;
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 初始化日志系统
    let _log_guard = init_logging(None);

    // 记录应用启动信息
    log_application_start();

    let com_init = unsafe { windows::Win32::System::Com::CoInitialize(None) };
    if com_init.is_err() {
        warn!("初始化com库失败：{:?}", com_init);
    }

    let builder = tauri::Builder::default().plugin(tauri_plugin_shell::init());
    builder
        .plugin(tauri_plugin_single_instance::init(|_app, _argv, _cwd| {
            error!("当前已经运行了一个实例");
            notify("zerolaunch-rs", "zerolaunch-rs已运行，不要重复运行");
        }))
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .manage(Arc::new(AppState::new()))
        .setup(|app| {
            tauri::async_runtime::block_on(async move {
                // 阶段1: 基础资源初始化（无依赖）
                info!("=== 阶段1: 基础资源初始化 ===");

                info!("正在注册图标路径");
                register_icon_path(app);

                // 阶段2: 核心状态初始化（依赖基础资源）
                info!("=== 阶段2: 核心状态初始化 ===");

                info!("正在初始化应用状态和配置系统");
                init_app_state(app).await;

                // 阶段3: UI组件初始化（依赖核心状态）
                info!("=== 阶段3: UI组件初始化 ===");

                info!("正在初始化搜索栏窗口");
                init_search_bar_window(app);

                info!("正在初始化设置窗口");
                init_setting_window(app.app_handle().clone());

                info!("正在初始化系统托盘服务");
                init_system_tray(app);

                // 阶段4: 交互服务初始化（依赖UI组件）
                info!("=== 阶段4: 交互服务初始化 ===");

                info!("正在启动快捷键管理器");
                start_shortcut_manager(app);

                // 阶段5: 配置应用和外部服务（依赖所有核心组件）
                info!("=== 阶段5: 配置应用和外部服务 ===");

                info!("正在更新应用设置");
                update_app_setting().await;

                info!("正在注册深度链接");
                app.deep_link()
                    .register_all()
                    .expect_programming("无法注册深度链接");
                info!("深度链接注册成功");

                app.deep_link().on_open_url(|event| {
                    let urls = event.urls();
                    debug!("收到深度链接事件: {:?}", urls);
                    tauri::async_runtime::spawn(async move {
                        let state = ServiceLocator::get_state();
                        let waiting_hashmap = state.get_waiting_hashmap();
                        for url in urls {
                            let domain = url.domain().expect_programming("URL缺少域名").to_string();
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
            launch_program_with_args,
            get_program_info,
            refresh_program,
            handle_search_text,
            get_launch_template_info,
            update_search_bar_window,
            get_background_picture,
            get_remote_config_dir,
            select_background_picture,
            hide_window,
            show_setting_window,
            show_welcome_window,
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
            command_get_latest_launch_program, //command_get_onedrive_refresh_token
            command_read_file,
            command_open_models_dir,
        ])
        .run(tauri::generate_context!())
        .expect_programming("error while running tauri application");
}

/// 初始化的流程-> 初始化程序的状态
async fn init_app_state(app: &mut App) {
    debug!("开始初始化应用状态");

    // === 阶段1: 核心状态初始化 ===
    let state = app.state::<Arc<AppState>>();
    ServiceLocator::init((*state).clone());
    debug!("ServiceLocator初始化完成");

    let state = ServiceLocator::get_state();

    // 立即设置app_handle，确保后续组件可以使用
    state.set_main_handle(Arc::new(app.app_handle().clone()));
    debug!("应用句柄设置完成");

    // === 阶段2: 存储管理器初始化 ===
    let create_and_show_welcome_page = move || {
        info!("第一次启动程序或者更新程序，创建欢迎页面");
        // 创建欢迎页面
        let welcome_result =
            tauri::WebviewWindowBuilder::new(app, "welcome", WebviewUrl::App("/welcome".into()))
                .title("欢迎下载ZeroLaunch-rs! 此页面只会出现一次，用于提供基础的使用说明╰(*°▽°*)╯")
                .visible(true)
                .drag_and_drop(false)
                .build();
        let welcome = Arc::new(match welcome_result {
            Err(e) => {
                error!("创建welcome页面失败: {:?}", e);
                return;
            }
            Ok(w) => w,
        });
        welcome
            .set_size(LogicalSize::new(950, 500))
            .expect_programming("无法设置欢迎窗口大小");
        // 监听welcome页面关闭事件，更新welcome页面版本
        welcome.on_window_event(move |event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                // 页面关闭时更新welcome页面版本
                update_welcome_page_version_on_close();
            }
        });
    };

    let storage_manager = StorageManager::new(create_and_show_welcome_page).await;
    // 立即设置存储管理器到状态中，使其可被其他组件使用
    state.set_storage_manager(Arc::new(storage_manager));
    debug!("存储管理器初始化并设置完成");

    // === 阶段3: 配置系统初始化 ===
    let storage_manager = state.get_storage_manager(); // 重新获取以使用Arc包装的版本
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

    let partial_config = load_string_to_runtime_config_(&remote_config_data);
    let runtime_config = RuntimeConfig::new();
    runtime_config.update(partial_config);

    // 立即设置配置到状态中
    state.set_runtime_config(Arc::new(runtime_config));
    debug!("运行时配置初始化并设置完成");

    // 立即应用日志级别配置
    let runtime_config = state.get_runtime_config();
    let app_config = runtime_config.get_app_config();
    let log_level = app_config.get_log_level();
    let tracing_level = tracing::Level::from(log_level);
    if let Err(e) = update_log_level(tracing_level) {
        warn!("更新日志级别失败: {}", e);
    } else {
        info!("日志级别已根据配置更新为: {:?}", tracing_level);
    }

    // === 阶段4: 程序管理器初始化 ===
    #[cfg(feature = "ai")]
    let model_manager = Arc::new(ModelManager::new());

    #[cfg(feature = "ai")]
    let embedding_backend = semantic_backend::create_embedding_backend(model_manager.clone());

    #[cfg(not(feature = "ai"))]
    let embedding_backend = semantic_backend::create_embedding_backend();

    let embedding_cache_bytes = if embedding_backend.is_some() {
        state
            .get_storage_manager()
            .download_file_bytes(SEMANTIC_EMBEDDING_CACHE_FILE_NAME.to_string())
            .await
    } else {
        None
    };

    let runtime_program_config = RuntimeProgramConfig {
        image_loader_config: RuntimeImageLoaderConfig {
            default_app_icon_path: APP_PIC_PATH
                .get("tips")
                .expect_programming("无法获取默认应用图标路径")
                .value()
                .clone(),
            default_web_icon_path: APP_PIC_PATH
                .get("web_pages")
                .expect_programming("无法获取默认网页图标路径")
                .value()
                .clone(),
        },
        embedding_backend,
        embedding_cache_bytes,
    };

    let program_manager = ProgramManager::new(runtime_program_config);
    // 立即设置程序管理器到状态中
    state.set_program_manager(Arc::new(program_manager));
    debug!("程序管理器初始化并设置完成");

    debug!("应用状态初始化完成");
}

/// 初始化搜索界面的窗口设置
fn init_search_bar_window(app: &mut App) {
    let main_window = Arc::new(
        app.get_webview_window("main")
            .expect_programming("无法获取主窗口"),
    );
    // 设置tauri窗口的大小等参数
    let monitor = main_window
        .current_monitor()
        .expect_programming("无法获取当前显示器")
        .expect_programming("显示器信息为空");
    // 获得了当前窗口的物理大小
    let size = monitor.size();
    let scale_factor = main_window.scale_factor().unwrap_or(1.0);
    let state = app.state::<Arc<AppState>>();
    let config = state.get_runtime_config();

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
        .expect_programming("无法获取资源目录")
        .join("icons");

    // 定义图标的键名和对应的文件名
    // (键名, 文件名)
    let icons_to_register = [
        ("tray_icon", "32x32.png"),
        ("tray_icon_white", "32x32-white.png"),
        ("web_pages", "web_pages.png"),
        ("tips", "tips.png"),
        ("terminal", "terminal.png"),
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
            .expect_programming("无法创建设置窗口"),
        );
        setting_window
            .set_size(LogicalSize::new(950, 500))
            .expect_programming("无法设置设置窗口大小");
        let window_clone = Arc::clone(&setting_window);
        setting_window.on_window_event(move |event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // 阻止窗口关闭
                api.prevent_close();
                // 隐藏窗口
                window_clone.hide().expect_programming("无法隐藏设置窗口");
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

    // 获取主窗口句柄用于发送事件
    let handle = state.get_main_handle();
    if let Err(e) = handle.emit("refresh_program_start", "") {
        tracing::debug!("emit refresh_program_start failed (may be expected during startup): {:?}", e);
    }

    let runtime_config = state.get_runtime_config();

    // 1.动态更新日志级别
    let app_config = runtime_config.get_app_config();
    let log_level = app_config.get_log_level();
    let tracing_level = tracing::Level::from(log_level);
    if let Err(e) = update_log_level(tracing_level) {
        warn!("更新日志级别失败: {}", e);
    } else {
        info!("日志级别已根据配置动态更新为: {:?}", tracing_level);
    }

    // 2. 重新更新程序索引的路径
    let program_manager = state.get_program_manager();
    let storage_manager = state.get_storage_manager();
    // 获取当前最新的描述信息的内容
    let semantic_store_str = match storage_manager
        .download_file_str(SEMANTIC_DESCRIPTION_FILE_NAME.to_string())
        .await
    {
        Some(data) => data,
        None => {
            // 如果没有获取到，则使用空的json对象，同时上传这个
            let ret = "{}".to_string();
            storage_manager
                .upload_file_str(SEMANTIC_DESCRIPTION_FILE_NAME.to_string(), ret.clone())
                .await;
            ret
        }
    };

    program_manager
        .load_from_config(
            runtime_config.get_program_manager_config(),
            Some(semantic_store_str),
        )
        .await;

    // 3. 判断要不要开机自启动
    if let Err(e) = handle_auto_start() {
        // 可以添加错误处理逻辑
        eprintln!("自启动设置失败: {:?}", e);
    }

    // 4.判断要不要静默启动
    handle_silent_start();

    // 5.判断要不要更新当前的窗口大小
    update_window_size_and_position();

    // 6.更新当前的窗口效果
    enable_window_effect();

    // 7.更新快捷键的绑定
    update_shortcut_manager();

        // 发送刷新结束事件
    if let Err(e) = handle.emit("refresh_program_end", "") {
        tracing::debug!("emit refresh_program_end failed: {:?}", e);
    }

    // 发送窗口更新事件
    if let Err(e) = handle.emit("update_search_bar_window", "") {
        eprintln!("发送窗口更新事件失败: {:?}", e);
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
    info!("开始保存配置文件, is_update_app: {}", is_update_app);

    let state = ServiceLocator::get_state();
    let runtime_config = state.get_runtime_config();
    debug!("获取运行时配置完成");

    let program_manager_runtime_data = state.get_program_manager().get_runtime_data().await;
    debug!("获取程序管理器运行时数据完成");
    let window = state
        .get_main_handle()
        .get_webview_window("main")
        .expect_programming("无法获取主窗口")
        .inner_position()
        .expect_programming("无法获取窗口位置");

    let partial_app_config = PartialAppConfig {
        window_position: Some((window.x, window.y)),
        ..Default::default()
    };

    runtime_config.update(PartialRuntimeConfig {
        app_config: Some(partial_app_config),
        ui_config: None,
        shortcut_config: None,
        program_manager_config: Some(program_manager_runtime_data.runtime_data),
        window_state: None,
    });
    let remote_config = runtime_config.to_partial();

    let data_str = save_runtime_config_to_string(remote_config);
    debug!("本地配置保存完成");

    let storage_manager = state.get_storage_manager();

    storage_manager
        .upload_file_str(REMOTE_CONFIG_NAME.to_string(), data_str)
        .await;
    //保存一下描述性信息
    storage_manager
        .upload_file_str(
            SEMANTIC_DESCRIPTION_FILE_NAME.to_string(),
            program_manager_runtime_data.semantic_store_str,
        )
        .await;
    if !program_manager_runtime_data.semantic_cache_bytes.is_empty() {
        storage_manager
            .upload_file_bytes(
                SEMANTIC_EMBEDDING_CACHE_FILE_NAME.to_string(),
                program_manager_runtime_data.semantic_cache_bytes,
            )
            .await;
    }
    debug!("远程配置上传完成");

    if is_update_app {
        update_app_setting().await;
    }
}

/// 处理自动开机的逻辑
pub fn handle_auto_start() -> Result<(), Box<dyn std::error::Error>> {
    info!("开始处理自动启动配置");

    let state: Arc<AppState> = ServiceLocator::get_state();

    // 处理主窗口句柄
    let app_handle = state.get_main_handle();

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
        let config = state.get_runtime_config();
        config.get_app_config().get_is_auto_start()
    };

    // 根据配置强制更新自动启动状态和路径
    if is_auto_start {
        info!("启用自动启动功能");
        // 无论当前是否启用，启用自动启动以确保路径正确
        autostart_manager.enable()?;
        debug!("自动启动已启用");
    } else {
        info!("检查并禁用自动启动功能");
        // 如果已启用则禁用
        if autostart_manager.is_enabled()? {
            debug!("检测到自动启动已启用，正在禁用");
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
        let runtime_config = state.get_runtime_config();
        let app_config = runtime_config.get_app_config();
        if !app_config.get_is_silent_start() {
            notify("ZeroLaunch-rs", "ZeroLaunch-rs已成功启动！");
        }
    });
}

/// 当welcome页面关闭时更新welcome页面版本
fn update_welcome_page_version_on_close() {
    tauri::async_runtime::spawn(async {
        let state = ServiceLocator::get_state();
        let storage_manager = state.get_storage_manager();
        // 获取当前welcome页面版本
        let current_version = storage::storage_manager::WELCOME_PAGE_VERSION.to_string();

        // 更新本地配置
        let partial_config = storage::config::PartialLocalConfig {
            storage_destination: None,
            local_save_config: None,
            webdav_save_config: None,
            save_to_local_per_update: None,
            version: None,
            welcome_page_version: Some(current_version),
        };

        storage_manager.update(partial_config).await;
        info!("已更新welcome页面版本到本地配置");
    });
}
