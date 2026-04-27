pub mod commands;
pub mod core;
pub mod error;
pub mod logging;
pub mod modules;
pub mod plugin;
pub mod plugin_system;
pub mod sdk;
pub mod state;
pub mod tray;
pub mod utils;
pub mod window_effect;
pub mod window_position;

use crate::core::config::components::hotkey_config::HotkeyConfigComponent;
use crate::core::config::components::storage_config::StorageConfigComponent;
use crate::core::config::ConfigManager;
use crate::logging::{init_logging, log_application_shutdown, log_application_start};
use crate::plugin::data_source::app_source::AppSource;
use crate::plugin::data_source::bookmark_source::BookmarkSource;
use crate::plugin::data_source::command_source::CommandSource;
use crate::plugin::data_source::program_source::ProgramSource;
use crate::plugin::data_source::url_source::UrlSource;
use crate::plugin::executor::{
    AppExecutor, CommandExecutor, FileExecutor, PathExecutor, UrlExecutor, WindowActivateExecutor,
};
use crate::plugin::keyword_optimizer::{
    FirstLetterExtractor, LowerCaseConverter, PinyinConverter, SpaceNormalizer, SpaceRemover,
    SymbolRemover, UpperCaseLetterExtractor, VersionNumberRemover,
};
use crate::plugin::score_booster::history_booster::HistoryBooster;
use crate::plugin::score_booster::query_affinity::QueryAffinityBooster;
use crate::plugin::search_engine::launchy_search_model::LaunchySearchModel;
use crate::plugin::search_engine::skim_search_model::SkimSearchModel;
use crate::plugin::search_engine::standard_search_model::StandardSearchModel;
use crate::plugin_system::types::{ScoreBooster, SearchEngine};
use crate::plugin_system::Configurable;
use crate::plugin_system::{CandidatePipeline, SearchPipeline};
use crate::sdk::path::KnownPath;
use crate::sdk::platform::WindowsAppEnumerator;
use crate::sdk::platform::WindowsAppLauncher;
use crate::sdk::platform::WindowsAutoStartManager;
use crate::sdk::platform::WindowsClipboardProvider;
use crate::sdk::platform::WindowsHotkeyManager;
use crate::sdk::platform::WindowsIconExtractor;
use crate::sdk::platform::WindowsLnkResolver;
use crate::sdk::platform::WindowsPathResolver;
use crate::sdk::platform::WindowsResourceLoader;
use crate::sdk::platform::WindowsSelectionProvider;
use crate::sdk::platform::WindowsShellExecutor;
use crate::sdk::platform::WindowsWindowHandleProvider;
use crate::sdk::platform::WindowsWindowManager;
use crate::sdk::storage::local_storage::LocalStorageService;
use crate::sdk::storage::storage_service::StorageService;
use crate::sdk::timer::TokioTimerManager;
use crate::sdk::AppResourceService;
use crate::sdk::PathResolver;
use crate::state::app_state::AppState;
use crate::tray::init_system_tray;
// use crate::utils::ui_controller::handle_focus_lost;
use crate::window_position::update_window_size_and_position;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::App;
use tauri::Emitter;
use tauri::LogicalSize;
use tauri::Manager;
use tauri::WebviewUrl;
use tauri_plugin_deep_link::DeepLinkExt;
use tracing::{debug, error, info, warn};
use utils::service_locator::ServiceLocator;
use windows::Win32::Foundation::POINT;
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

static IS_EXITING: AtomicBool = AtomicBool::new(false);

pub async fn do_cleanup_before_exit() {
    info!("执行退出前清理工作...");
    let state = ServiceLocator::get_state();
    state.get_config_manager().save_to_storage();
    log_application_shutdown();
    info!("退出前清理工作完成");
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // ========================================================================
    // 阶段 1: 初始化日志系统
    // ========================================================================
    let path_resolver = Arc::new(WindowsPathResolver::new());
    let log_dir = path_resolver.resolve_path(KnownPath::AppLogDir).unwrap();
    let _log_guard = init_logging(&log_dir, None);
    log_application_start();

    // 初始化 COM 库
    let com_init = unsafe { windows::Win32::System::Com::CoInitialize(None) };
    if com_init.is_err() {
        warn!("初始化COM库失败：{:?}", com_init);
    }

    // ========================================================================
    // 阶段 3: 构建 Tauri 应用
    // ========================================================================

    let builder = tauri::Builder::default().plugin(tauri_plugin_shell::init());
    builder
        .plugin(tauri_plugin_single_instance::init(|_app, _argv, _cwd| {
            error!("当前已经运行了一个实例");
            todo!("通知已经运行一个实例");
        }))
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .manage(Arc::new(AppState::new()))
        .setup(move |app| {
            let app_data_dir = path_resolver.resolve_path(KnownPath::AppDataDir).unwrap();
            let icon_cache_dir = path_resolver
                .resolve_path(KnownPath::AppIconCacheDir)
                .unwrap();
            let config_dir = path_resolver.resolve_path(KnownPath::AppConfigDir).unwrap();

            tauri::async_runtime::block_on(async move {
                info!("=== 阶段5: 核心状态初始化 ===");
                info!("正在初始化应用状态和配置系统");
                init_app_state(app, path_resolver, app_data_dir, icon_cache_dir, config_dir).await;

                info!("=== 阶段6: UI组件初始化 ===");
                info!("正在初始化搜索栏窗口");
                init_search_bar_window(app);

                info!("正在初始化设置窗口");
                init_setting_window(app.app_handle().clone());

                info!("正在初始化系统托盘服务");
                init_system_tray(app).await;

                info!("正在注册深度链接");
                app.deep_link().register_all().expect("无法注册深度链接");
                info!("深度链接注册成功");

                app.deep_link().on_open_url(|event| {
                    let urls = event.urls();
                    debug!("收到深度链接事件: {:?}", urls);
                    tauri::async_runtime::spawn(async move {
                        let state = ServiceLocator::get_state();
                        let waiting_hashmap = state.get_waiting_hashmap();
                        for url in urls {
                            let domain = url.domain().expect("URL缺少域名").to_string();
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
            // Bridge: 搜索与会话管理
            crate::commands::bridge::bridge_query,
            crate::commands::bridge::bridge_confirm,
            crate::commands::bridge::bridge_wake,
            crate::commands::bridge::bridge_reset,
            crate::commands::bridge::bridge_get_session_mode,
            crate::commands::bridge::bridge_refresh_candidates,
            crate::commands::bridge::bridge_get_candidates_count,
            // Bridge: 配置管理
            crate::commands::config_file::config_get_all_components,
            crate::commands::config_file::config_get_schema,
            crate::commands::config_file::config_get_settings,
            crate::commands::config_file::config_apply_settings,
            crate::commands::config_file::config_reset_settings,
            crate::commands::config_file::config_set_enabled,
            crate::commands::config_file::config_get_actions,
            crate::commands::config_file::config_execute_action,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| match event {
            tauri::RunEvent::ExitRequested { api, .. } => {
                if !IS_EXITING.load(Ordering::Relaxed) {
                    info!("检测到退出请求，开始清理...");
                    api.prevent_exit();
                    IS_EXITING.store(true, Ordering::Relaxed);

                    let app_handle = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        do_cleanup_before_exit().await;
                        info!("清理完成，正在退出程序...");
                        app_handle.exit(0);
                    });
                }
            }
            tauri::RunEvent::WindowEvent { label, event, .. } => {
                if label == "main" {
                    if let tauri::WindowEvent::ThemeChanged(theme) = event {
                        crate::tray::update_tray_icon_theme();
                        let theme_str = match theme {
                            tauri::Theme::Dark => "dark",
                            tauri::Theme::Light => "light",
                            _ => "light",
                        };
                        let _ = app_handle.emit("system-theme-changed", theme_str);
                    }
                }
            }
            _ => {}
        });
}

async fn init_app_state(
    app: &mut App,
    path_resolver: Arc<WindowsPathResolver>,
    app_data_dir: String,
    icon_cache_dir: String,
    config_dir: String,
) {
    debug!("开始初始化应用状态");

    let state = app.state::<Arc<AppState>>();
    ServiceLocator::init((*state).clone());
    debug!("ServiceLocator初始化完成");

    let state = ServiceLocator::get_state();

    state.set_main_handle(Arc::new(app.app_handle().clone()));
    debug!("应用句柄设置完成");

    // 初始化应用资源服务（图标等内置资源）
    let resource_dir = app.path().resource_dir().expect("无法获取资源目录");
    let icons_dir = resource_dir.join("icons");
    let app_resource = Arc::new(AppResourceService::new(
        icons_dir.to_string_lossy().to_string(),
    ));

    info!("=== 阶段5.1: 创建 HostApi ===");

    let default_storage: Arc<dyn StorageService> =
        Arc::new(LocalStorageService::new(&app_data_dir));

    let default_app_icon_path = app_resource
        .get_icon_path("tips")
        .unwrap_or_else(|| ".".to_string());
    let default_web_icon_path = app_resource
        .get_icon_path("web_pages")
        .unwrap_or_else(|| ".".to_string());

    let app_handle = state.get_main_handle();
    let host_api = Arc::new(
        crate::sdk::HostApi::builder(icon_cache_dir)
            .icon_extractor(Arc::new(WindowsIconExtractor::new(
                default_app_icon_path,
                default_web_icon_path,
            )))
            .shell_executor(Arc::new(WindowsShellExecutor::new()))
            .window_manager(Arc::new(WindowsWindowManager::new()))
            .path_resolver(path_resolver)
            .app_enumerator(Arc::new(WindowsAppEnumerator::new()))
            .app_launcher(Arc::new(WindowsAppLauncher::new()))
            .lnk_resolver(Arc::new(WindowsLnkResolver::new()))
            .resource_loader(Arc::new(WindowsResourceLoader::new()))
            .parameter_resolver(Arc::new(
                crate::sdk::parameter::DefaultParameterResolver::new(),
            ))
            .parameter_providers(
                Arc::new(WindowsClipboardProvider),
                Arc::new(WindowsWindowHandleProvider),
                Arc::new(WindowsSelectionProvider),
            )
            .autostart_manager(Arc::new(WindowsAutoStartManager::new()))
            .hotkey_manager(Arc::new(WindowsHotkeyManager::new(app_handle)))
            .timer_manager(Arc::new(TokioTimerManager::new()))
            .storage_service(default_storage)
            .app_resource(app_resource)
            .build(),
    );
    state.set_host_api(host_api.clone());
    info!("HostApi 初始化完成");

    let config_manager = Arc::new(ConfigManager::new(std::path::PathBuf::from(&config_dir)));
    config_manager.set_host_api(host_api.clone());
    let storage_config_component = Arc::new(StorageConfigComponent::new(host_api.clone()));
    config_manager.register(storage_config_component);
    state.set_config_manager(config_manager);
    info!("ConfigManager 初始化完成");

    info!("=== 阶段5.2: 新插件系统初始化 ===");
    init_plugin_system(&state);
    debug!("应用状态初始化完成");
}

fn init_plugin_system(state: &Arc<AppState>) {
    let session_router = state.get_session_router();
    let config_manager = state.get_config_manager();

    session_router.set_config_manager(config_manager.clone());

    let event_router = session_router.clone();
    let mut event_receiver = config_manager.event_sender().subscribe();
    tauri::async_runtime::spawn(async move {
        loop {
            match event_receiver.recv().await {
                Ok(event) => {
                    event_router.handle_config_event(&event);
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(count)) => {
                    warn!("配置事件接收器落后 {} 条消息", count);
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    info!("配置事件通道已关闭，退出监听");
                    break;
                }
            }
        }
    });

    info!("正在注册可配置组件到 ConfigManager...");

    let host_api = state.get_host_api();
    session_router.set_host_api(host_api.clone());

    let shell_service_handle = host_api.register("shell-executors", Default::default());
    let window_service_handle = host_api.register("window-activator", Default::default());
    let program_source_handle = host_api.register("program-source", Default::default());
    let app_source_handle = host_api.register("app-source", Default::default());
    let app_executor_handle = host_api.register("app-executor", Default::default());
    let command_executor_handle = host_api.register("command-executor", Default::default());
    let url_source_handle = host_api.register("url-source", Default::default());
    let bookmark_source_handle = host_api.register("bookmark-source", Default::default());
    let command_source_handle = host_api.register("command-source", Default::default());

    // 1. 注册执行器（同时注册到 ConfigManager 和 ExecutorRegistry，双重索引）
    info!("正在注册执行器...");
    let path_executor: Arc<dyn crate::plugin_system::types::ActionExecutor> =
        Arc::new(PathExecutor::new(shell_service_handle.clone()));
    let file_executor: Arc<dyn crate::plugin_system::types::ActionExecutor> =
        Arc::new(FileExecutor::new(shell_service_handle.clone()));
    let url_executor: Arc<dyn crate::plugin_system::types::ActionExecutor> =
        Arc::new(UrlExecutor::new(shell_service_handle.clone()));
    let app_executor: Arc<dyn crate::plugin_system::types::ActionExecutor> =
        Arc::new(AppExecutor::new(app_executor_handle));
    let command_executor: Arc<dyn crate::plugin_system::types::ActionExecutor> =
        Arc::new(CommandExecutor::new(command_executor_handle));
    let window_activate_executor: Arc<dyn crate::plugin_system::types::ActionExecutor> =
        Arc::new(WindowActivateExecutor::new(window_service_handle));

    // 注册到 ConfigManager（配置维度索引）
    config_manager.register(path_executor.clone());
    config_manager.register(file_executor.clone());
    config_manager.register(url_executor.clone());
    config_manager.register(app_executor.clone());
    config_manager.register(command_executor.clone());
    config_manager.register(window_activate_executor.clone());

    // 注册到 ExecutorRegistry（业务维度索引）
    // 注意，这个步骤应该交给config_manager来完成，因为要只给session_router注册已启用的executor，对于没有启用的，那么是不应该注册的！当前的实现下，先默认都启用，为了方便测试，则直接在这里完成注册！
    session_router.register_executor(path_executor);
    session_router.register_executor(file_executor);
    session_router.register_executor(url_executor);
    session_router.register_executor(app_executor);
    session_router.register_executor(command_executor);
    session_router.register_executor(window_activate_executor);
    info!("执行器注册完成");

    // 2. 注册数据源
    info!("正在注册数据源...");
    let program_source = Arc::new(ProgramSource::new(program_source_handle));
    let _ = program_source.apply_settings(program_source.get_default_settings());
    let app_source = Arc::new(AppSource::new(app_source_handle));
    let url_source = Arc::new(UrlSource::new(url_source_handle));
    let bookmark_source = Arc::new(BookmarkSource::new(bookmark_source_handle));
    let command_source = Arc::new(CommandSource::new(command_source_handle));
    config_manager.register(program_source.clone());
    config_manager.register(app_source.clone());
    config_manager.register(url_source.clone());
    config_manager.register(bookmark_source.clone());
    config_manager.register(command_source.clone());
    info!("数据源注册完成");

    // 3. 注册关键词优化器
    info!("正在注册关键词优化器...");
    let version_number_remover = Arc::new(VersionNumberRemover::new());
    let symbol_remover = Arc::new(SymbolRemover::new());
    let space_remover = Arc::new(SpaceRemover::new());
    let space_normalizer = Arc::new(SpaceNormalizer::new());
    let lower_case_converter = Arc::new(LowerCaseConverter::new());
    let pinyin_converter = Arc::new(PinyinConverter::new());
    let first_letter_extractor = Arc::new(FirstLetterExtractor::new());
    let upper_case_letter_extractor = Arc::new(UpperCaseLetterExtractor::new());

    config_manager.register(version_number_remover.clone());
    config_manager.register(symbol_remover.clone());
    config_manager.register(space_remover.clone());
    config_manager.register(space_normalizer.clone());
    config_manager.register(lower_case_converter.clone());
    config_manager.register(pinyin_converter.clone());
    config_manager.register(first_letter_extractor.clone());
    config_manager.register(upper_case_letter_extractor.clone());
    info!("关键词优化器注册完成");

    // 4. 注册搜索引擎
    info!("正在注册搜索引擎...");
    let search_engine: Arc<dyn SearchEngine> = Arc::new(StandardSearchModel {});
    let launchy_search_engine: Arc<dyn SearchEngine> = Arc::new(LaunchySearchModel {});
    let skim_search_engine: Arc<dyn SearchEngine> = Arc::new(SkimSearchModel::new());
    config_manager.register(search_engine.clone());
    config_manager.register(launchy_search_engine);
    config_manager.register(skim_search_engine);
    info!("搜索引擎注册完成");

    // 5. 注册分数增强器
    info!("正在注册分数增强器...");
    let history_booster: Arc<dyn ScoreBooster> = Arc::new(HistoryBooster::new());
    let query_affinity_booster: Arc<dyn ScoreBooster> = Arc::new(QueryAffinityBooster::new());
    config_manager.register(history_booster.clone());
    config_manager.register(query_affinity_booster.clone());
    info!("分数增强器注册完成");

    // 6. 注册核心配置组件
    info!("正在注册核心配置组件...");
    let hotkey_config_component = Arc::new(HotkeyConfigComponent::new(host_api.clone()));
    config_manager.register(hotkey_config_component);
    info!("核心配置组件注册完成");

    // === 阶段2: 从 ConfigManager 按类型获取组件，构建各 Pipeline ===
    info!("正在构建业务管道...");

    // 构建候选管道
    let mut candidate_pipeline = CandidatePipeline::new();

    // 添加数据源
    candidate_pipeline.add_source(program_source);
    candidate_pipeline.add_source(app_source);
    candidate_pipeline.add_source(url_source);
    candidate_pipeline.add_source(bookmark_source);
    candidate_pipeline.add_source(command_source);

    // 添加关键词优化器（按优先级顺序）
    candidate_pipeline.add_keyword_optimizer(version_number_remover);
    candidate_pipeline.add_keyword_optimizer(symbol_remover);
    candidate_pipeline.add_keyword_optimizer(space_remover);
    candidate_pipeline.add_keyword_optimizer(space_normalizer);
    candidate_pipeline.add_keyword_optimizer(lower_case_converter);
    candidate_pipeline.add_keyword_optimizer(pinyin_converter);
    candidate_pipeline.add_keyword_optimizer(first_letter_extractor);
    candidate_pipeline.add_keyword_optimizer(upper_case_letter_extractor);

    // 收集候选项
    info!("正在收集候选项...");
    let candidates = candidate_pipeline.collect();
    info!(
        "候选项收集完成，共 {} 个",
        candidates.get_candidates().len()
    );

    // 构建搜索管道
    let boosters: Vec<Arc<dyn ScoreBooster>> = vec![history_booster, query_affinity_booster];
    let search_pipeline = SearchPipeline::new(Some(search_engine), boosters, 10);

    // === 阶段3: 更新 SessionRouter 状态 ===
    info!("正在更新 SessionRouter 状态...");
    session_router.set_candidate_pipeline(candidate_pipeline);
    session_router.set_search_pipeline(search_pipeline);
    session_router.set_cached_candidates(candidates);

    // === 阶段4: 加载持久化配置 ===
    info!("正在加载持久化配置...");
    if let Err(e) = config_manager.load_from_storage() {
        warn!("加载持久化配置失败: {}", e);
    }

    info!(
        "新插件系统初始化完成，已缓存 {} 个候选项",
        session_router.get_cached_candidates_count()
    );
}

/// 初始化搜索界面的窗口设置
fn init_search_bar_window(app: &mut App) {
    let main_window = Arc::new(app.get_webview_window("main").expect("无法获取主窗口"));
    let monitor = main_window
        .current_monitor()
        .expect("无法获取当前显示器")
        .expect("显示器信息为空");
    let _size = monitor.size();
    let _scale_factor = main_window.scale_factor().unwrap_or(1.0);
    let _state = app.state::<Arc<AppState>>();

    update_window_size_and_position();
    // 设置当窗口被关闭时，忽略
    let windows_clone = main_window.clone();
    main_window.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            // handle_focus_lost(windows_clone.clone());
            todo!("实现处理窗口失焦的场景");
        }
        if let tauri::WindowEvent::Focused(focused) = event {
            if !focused {
                let mut point = POINT { x: 0, y: 0 };
                unsafe {
                    let _ = GetCursorPos(&mut point);
                }
                let position = (point.x, point.y);
                if let Ok(window_position) = windows_clone.inner_position() {
                    if let Ok(window_size) = windows_clone.inner_size() {
                        let in_window = position.0 >= window_position.x
                            && position.0 <= window_position.x + window_size.width as i32
                            && position.1 >= window_position.y
                            && position.1 <= window_position.y + window_size.height as i32;
                        if !in_window {
                            // handle_focus_lost(windows_clone.clone());
                            todo!("实现处理窗口失焦的场景");
                        }
                    }
                }
            }
        }
    });
    // handle_focus_lost(main_window.clone());
    todo!("实现处理窗口失焦的场景");
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
            .expect("无法创建设置窗口"),
        );
        setting_window
            .set_size(LogicalSize::new(950, 500))
            .expect("无法设置设置窗口大小");
        let window_clone = Arc::clone(&setting_window);
        setting_window.on_window_event(move |event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                window_clone.hide().expect("无法隐藏设置窗口");
                debug!("隐藏设置窗口");
            }
        });
    });
}
