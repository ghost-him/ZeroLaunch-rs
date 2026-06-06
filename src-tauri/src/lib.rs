pub mod cli_server;
pub mod commands;
pub mod core;
pub mod logging;
pub mod plugin;
pub mod plugin_loader;
pub mod plugin_protocol_assets;
pub mod plugin_system;
pub mod sdk;
pub mod state;
pub mod utils;

use crate::core::config::{ConfigEvent, ConfigManager};
use crate::core::tray::TrayManager;
use crate::logging::{init_logging, log_application_shutdown, log_application_start};

use crate::plugin_system::CandidatePipeline;
use crate::sdk::host_api::HostApi;
use crate::sdk::HostApiBuilder;
use crate::state::app_state::AppState;
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
use zerolaunch_platform_windows::windows_capabilities;
use zerolaunch_platform_windows::ComGuard;
use zerolaunch_platform_windows::WindowsAppEnumerator;
use zerolaunch_platform_windows::WindowsAppLauncher;
use zerolaunch_platform_windows::WindowsAutoStartManager;
use zerolaunch_platform_windows::WindowsClipboardProvider;
use zerolaunch_platform_windows::WindowsFocusMonitor;
use zerolaunch_platform_windows::WindowsHotkeyManager;
use zerolaunch_platform_windows::WindowsIconExtractor;
use zerolaunch_platform_windows::WindowsInstallationMonitor;
use zerolaunch_platform_windows::WindowsLnkResolver;
use zerolaunch_platform_windows::WindowsPathResolver;
use zerolaunch_platform_windows::WindowsResourceLoader;
use zerolaunch_platform_windows::WindowsSelectionProvider;
use zerolaunch_platform_windows::WindowsShellExecutor;
use zerolaunch_platform_windows::WindowsWindowHandleProvider;
use zerolaunch_platform_windows::WindowsWindowManager;
use zerolaunch_platform_windows::WindowsWindowPositioner;
use zerolaunch_plugin_api::services::hotkey::types::HotkeyEventFilter;
use zerolaunch_plugin_api::services::path::KnownPath;
use zerolaunch_plugin_api::services::storage::local_storage::LocalStorageService;
use zerolaunch_plugin_api::services::storage::storage_service::StorageService;
use zerolaunch_plugin_api::services::timer::TokioTimerManager;
use zerolaunch_plugin_api::services::window::{MonitorInfo, PositionRequest, WindowPosition};
use zerolaunch_plugin_api::services::AppResourceService;
use zerolaunch_plugin_api::services::PathResolver;
use zerolaunch_plugin_host::manager::PluginHostManager;
static IS_EXITING: AtomicBool = AtomicBool::new(false);

pub async fn do_cleanup_before_exit() {
    info!("执行退出前清理工作...");
    let state = ServiceLocator::get_state();
    if let Err(e) = state.get_config_manager().save_to_storage() {
        warn!("退出前配置保存失败: {}", e);
    }
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

    // 初始化 COM 库（主线程常驻，不释放）
    let _com_guard = unsafe { ComGuard::init() };
    std::mem::forget(_com_guard);

    // ========================================================================
    // 阶段 3: 构建 Tauri 应用
    // ========================================================================

    let builder = tauri::Builder::default().plugin(tauri_plugin_shell::init());
    builder
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            use tauri_plugin_notification::NotificationExt;
            let _ = app
                .notification()
                .builder()
                .title("ZeroLaunch")
                .body("程序已在运行中，无需重复启动")
                .show();
        }))
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .register_uri_scheme_protocol("zlplugin", move |_app, request| {
            let uri = request.uri().to_string();
            match crate::plugin_protocol_assets::handler::handle(&uri) {
                Ok((bytes, mime)) => {
                    let mut response = http::Response::new(bytes);
                    response.headers_mut().insert(
                        http::header::CONTENT_TYPE,
                        http::HeaderValue::from_str(&mime).unwrap(),
                    );
                    response
                }
                Err(e) => {
                    let msg = format!("zlplugin error: {}", e);
                    http::Response::builder()
                        .status(http::StatusCode::FORBIDDEN)
                        .body(msg.into_bytes())
                        .unwrap()
                }
            }
        })
        .manage(Arc::new(AppState::new()))
        .setup(move |app| {
            let app_data_dir = path_resolver.resolve_path(KnownPath::AppDataDir).unwrap();
            let icon_cache_dir = path_resolver
                .resolve_path(KnownPath::AppIconCacheDir)
                .unwrap();
            let config_dir = path_resolver.resolve_path(KnownPath::AppConfigDir).unwrap();

            info!("应用数据目录: {}", app_data_dir);
            info!("图标缓存目录: {}", icon_cache_dir);
            info!("配置目录: {}", config_dir);

            tauri::async_runtime::block_on(async move {
                info!("=== 核心服务初始化 ===");
                init_app_state(app, path_resolver, app_data_dir, icon_cache_dir, config_dir).await;

                info!("=== UI 组件初始化 ===");
                info!("正在初始化搜索栏窗口");
                init_search_bar_window(app);

                info!("正在初始化设置窗口");
                init_setting_window(app.app_handle().clone());

                info!("正在初始化系统托盘");
                let tray_manager = app
                    .state::<Arc<AppState>>()
                    .get_tray_manager()
                    .expect("TrayManager not initialized");
                tray_manager.init(app).await;

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

                info!("应用启动完成");
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
            crate::commands::bridge::bridge_hide_window,
            // Bridge: 配置管理
            crate::commands::config_file::config_get_all_components,
            crate::commands::config_file::config_get_schema,
            crate::commands::config_file::config_get_settings,
            crate::commands::config_file::config_apply_settings,
            crate::commands::config_file::config_reset_settings,
            crate::commands::config_file::config_set_enabled,
            crate::commands::config_file::config_get_actions,
            crate::commands::config_file::config_execute_action,
            // 资源管理
            crate::commands::resource::resource_get,
            crate::commands::resource::resource_upload,
            // Plugin Inspector
            crate::commands::inspector::inspector_get_state,
            crate::commands::inspector::inspector_simulate_query,
            // Third-party Plugin Management
            crate::commands::plugin::plugin_list,
            crate::commands::plugin::plugin_get_manifest,
            crate::commands::plugin::plugin_install_local,
            crate::commands::plugin::plugin_reload,
            crate::commands::plugin::plugin_uninstall,
            crate::commands::plugin::plugin_set_enabled,
            crate::commands::plugin::plugin_get_logs,
            crate::commands::cli::cli_get_info,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| match event {
            tauri::RunEvent::ExitRequested { api, .. } if !IS_EXITING.load(Ordering::Relaxed) => {
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
            tauri::RunEvent::WindowEvent {
                label,
                event: tauri::WindowEvent::ThemeChanged(theme),
                ..
            } if label == "main" => {
                if let Some(tray_manager) = app_handle.state::<Arc<AppState>>().get_tray_manager() {
                    tray_manager.update_icon_theme();
                }
                let theme_str = match theme {
                    tauri::Theme::Dark => "dark",
                    tauri::Theme::Light => "light",
                    _ => "light",
                };
                let _ = app_handle.emit("system-theme-changed", theme_str);
            }
            _ => {}
        });
}

/// 配置 HostApiBuilder，注入所有 Windows 平台实现以及平台无关的默认组件。
/// 返回预配置的 HostApiBuilder，调用方继续添加 Tauri 相关回调后调用 build()。
fn build_windows_host_api_builder(
    icon_cache_dir: String,
    default_app_icon_path: String,
    default_web_icon_path: String,
    path_resolver: Arc<dyn PathResolver>,
    default_storage: Arc<dyn StorageService>,
    app_resource: Arc<AppResourceService>,
) -> HostApiBuilder {
    HostApi::builder(icon_cache_dir)
        .capabilities(windows_capabilities())
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
            zerolaunch_plugin_api::services::parameter::DefaultParameterResolver::new(),
        ))
        .parameter_providers(
            Arc::new(WindowsClipboardProvider),
            Arc::new(WindowsWindowHandleProvider),
            Arc::new(WindowsSelectionProvider),
        )
        .autostart_manager(Arc::new(WindowsAutoStartManager::new()))
        .installation_monitor(Arc::new(WindowsInstallationMonitor::new()))
        .timer_manager(Arc::new(TokioTimerManager::new()))
        .storage_service(default_storage)
        .app_resource(app_resource)
        .window_positioner(Arc::new(WindowsWindowPositioner::new()))
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

    info!("=== Phase 1: SDK 初始化 - 创建 HostApi ===");

    let default_storage: Arc<dyn StorageService> =
        Arc::new(LocalStorageService::new(&app_data_dir));

    let default_app_icon_path = app_resource
        .get_icon_path("tips")
        .unwrap_or_else(|| ".".to_string());
    let default_web_icon_path = app_resource
        .get_icon_path("web_pages")
        .unwrap_or_else(|| ".".to_string());

    let app_handle = state.get_main_handle();
    let app_handle_for_notify = app_handle.clone();
    let app_handle_for_hide = app_handle.clone();
    let app_handle_for_show = app_handle.clone();
    let app_handle_for_is_visible = app_handle.clone();
    let app_handle_for_focus_monitor = app_handle.clone();
    let app_handle_for_set_pos = app_handle.clone();

    let host_api = Arc::new(
        build_windows_host_api_builder(
            icon_cache_dir,
            default_app_icon_path,
            default_web_icon_path,
            path_resolver,
            default_storage,
            app_resource,
        )
        .hotkey_manager(Arc::new(WindowsHotkeyManager::new(app_handle)))
        .focus_monitor(Arc::new(WindowsFocusMonitor::new(
            app_handle_for_focus_monitor,
        )))
        .set_window_position_callback(move |x, y| {
            if let Some(window) = app_handle_for_set_pos.get_webview_window("main") {
                let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
            }
        })
        .notify_callback(move |title: String, message: String| {
            use tauri_plugin_notification::NotificationExt;
            let _ = app_handle_for_notify
                .notification()
                .builder()
                .title(title)
                .body(message)
                .show();
        })
        .hide_window_callback(move || {
            if let Some(window) = app_handle_for_hide.get_webview_window("main") {
                let _ = window.hide();
                let _ = window.emit("handle_focus_lost", ());
            }
        })
        .show_window_callback(move || {
            if let Some(window) = app_handle_for_show.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
                let _ = window.emit("show_window", ());
            }
        })
        .is_window_visible_callback(move || {
            app_handle_for_is_visible
                .get_webview_window("main")
                .map(|w| w.is_visible().unwrap_or(false))
                .unwrap_or(false)
        })
        .build()
        .expect("Failed to build HostApi"),
    );
    state.set_host_api(host_api.clone());
    info!("HostApi 初始化完成");

    let core_handle = host_api.register("core", Default::default());
    state.set_core_handle(core_handle.clone());
    info!("Core PluginHandle 注册完成");

    #[cfg(feature = "inspector")]
    {
        use crate::plugin_system::inspector::Inspector;
        state.set_inspector(Arc::new(Inspector::new(200)));
        info!("Plugin Inspector 已启用 (容量: 200)");
    }

    let tray_manager = Arc::new(TrayManager::new(host_api.clone()));
    state.set_tray_manager(tray_manager);
    info!("TrayManager 创建完成");

    info!("=== Phase 2: Core 初始化 - 创建 ConfigManager ===");

    let config_manager = Arc::new(ConfigManager::new(std::path::PathBuf::from(&config_dir)));
    config_manager.set_host_api(host_api.clone());
    state.set_config_manager(config_manager);
    info!("ConfigManager 初始化完成");

    info!("=== Phase 3: Plugin 初始化 ===");
    init_plugin_system(&state).await;

    info!("=== Phase 4: 第三方插件加载 ===");
    let plugins_dir = std::path::PathBuf::from(&app_data_dir).join("plugins");
    let plugin_data_dir = std::path::PathBuf::from(&app_data_dir).join("plugin-data");
    let plugin_log_dir = std::path::PathBuf::from(&app_data_dir).join("plugin-logs");

    // Set plugins dir for zlplugin:// protocol handler
    crate::plugin_protocol_assets::handler::set_plugins_dir(plugins_dir.clone());

    let plugin_host_manager = Arc::new(PluginHostManager::new(plugin_data_dir, plugin_log_dir));
    state.set_plugin_host_manager(plugin_host_manager.clone());

    crate::plugin_loader::loader::load_all(
        &plugins_dir,
        state.get_config_manager(),
        state.get_session_router().clone(),
        plugin_host_manager,
        state.get_host_api(),
        app.handle().clone(),
    )
    .await;

    // Start CLI HTTP server
    info!("启动 CLI HTTP 服务器...");
    let cli_handle =
        crate::cli_server::server::start(state.clone(), &std::path::PathBuf::from(&app_data_dir))
            .await;
    match cli_handle {
        Ok(handle) => info!("CLI HTTP 服务器已启动于 127.0.0.1:{}", handle.port),
        Err(e) => tracing::warn!("CLI HTTP 服务器启动失败: {}", e),
    }

    info!(
        "应用状态初始化完成 (HostApi, ConfigManager, {} 个已注册组件)",
        state.get_config_manager().get_all_components().len()
    );
}

async fn init_plugin_system(state: &Arc<AppState>) {
    let session_router = state.get_session_router();
    let config_manager = state.get_config_manager();

    session_router.set_config_manager(config_manager.clone());

    // 订阅配置事件
    let event_router = session_router.clone();
    let app_handle = state.get_main_handle();
    let mut event_receiver = config_manager.event_sender().subscribe();
    tauri::async_runtime::spawn(async move {
        loop {
            match event_receiver.recv().await {
                Ok(event) => {
                    event_router.handle_config_event(&event).await;
                    // 将 SettingsChanged 事件桥接到 Tauri 前端，实现跨窗口同步。
                    // 注：Registered/Unregistered 仅启动时触发（前端窗口未创建），
                    // EnabledChanged 暂无前端消费者，故暂不转发。
                    if let ConfigEvent::SettingsChanged {
                        component_id,
                        component_type,
                    } = &event
                    {
                        let _ = app_handle.emit(
                            "config-changed",
                            serde_json::json!({
                                "componentId": component_id,
                                "componentType": format!("{:?}", component_type),
                            }),
                        );
                    }
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

    let host_api = state.get_host_api();
    session_router.set_host_api(host_api.clone());

    // ========================================================================
    // Phase A: inventory 自动发现并注册所有内置组件
    // ========================================================================
    info!("=== Phase A: inventory 自动发现并注册所有内置组件 ===");

    let ctx = crate::plugin_system::builtin_registry::InventoryContext::new(host_api.clone());
    let (data_sources, keyword_optimizers) =
        crate::plugin_system::builtin_registry::register_all_builtin_components(
            &ctx,
            &config_manager,
            session_router,
        );

    info!(
        "Phase A 完成: 共注册 {} 个组件",
        config_manager.get_all_components().len(),
    );

    // 注册快捷键回调：按下全局快捷键时切换搜索栏显示/隐藏
    let core_handle_for_hotkey = state.get_core_handle();
    let host_api_for_hotkey = host_api.clone();
    let session_router_for_hotkey = session_router.clone();
    let config_manager_for_hotkey = config_manager.clone();
    let app_handle_for_hotkey = state.get_main_handle();
    core_handle_for_hotkey.register_hotkey_callback(
        "search_bar_toggle",
        HotkeyEventFilter::All,
        Arc::new(move |event| {
            debug!("收到快捷键事件: {:?}", event);
            let host_api = host_api_for_hotkey.clone();
            let session_router = session_router_for_hotkey.clone();
            let config_manager = config_manager_for_hotkey.clone();
            let app_handle = app_handle_for_hotkey.clone();
            tauri::async_runtime::spawn(async move {
                if host_api.is_window_visible() {
                    save_window_position_if_drag(&config_manager, &app_handle);
                    host_api.hide_window().await;
                } else {
                    if !prepare_window_position(&config_manager, &host_api, &app_handle).await {
                        return;
                    }
                    let _ = session_router.on_search_bar_wake().await;
                    host_api.show_window().await;
                }
            });
        }),
    );

    // ========================================================================
    // Phase B: 加载持久化配置
    // ========================================================================
    info!("=== Phase B: 加载持久化配置 ===");
    if let Err(e) = config_manager.load_from_storage(true).await {
        warn!("加载持久化配置失败: {}", e);
    }

    // ========================================================================
    // Phase C: 构建管道
    // ========================================================================
    info!("=== Phase C: 构建业务管道 ===");

    info!("构建候选管道...");
    let mut candidate_pipeline = CandidatePipeline::new();
    for source in &data_sources {
        candidate_pipeline.add_source(source.clone());
    }
    for optimizer in &keyword_optimizers {
        candidate_pipeline.add_keyword_optimizer(optimizer.clone());
    }

    info!("正在收集候选项（此时各组件已持有用户持久化配置）...");
    let candidates = candidate_pipeline.collect().await;
    info!(
        "候选项收集完成，共 {} 个",
        candidates.get_candidates().len()
    );

    info!("根据已注册且启用的搜索引擎与增强器重建搜索管道...");
    session_router.rebuild_search_pipeline();

    info!("更新 SessionRouter 状态...");
    session_router
        .set_candidate_pipeline(candidate_pipeline)
        .await;
    session_router.set_cached_candidates(candidates);

    info!(
        "插件系统初始化完成，已注册 {} 个组件，缓存 {} 个候选项",
        config_manager.get_all_components().len(),
        session_router.get_cached_candidates_count()
    );
}

/// 准备搜索栏窗口位置：全屏检查 → 读取定位配置 → 计算并设置窗口坐标。
///
/// 返回 `true` 表示定位成功可继续唤醒；
/// 返回 `false` 表示被阻拦（全屏应用且未开启全屏唤醒）。
async fn prepare_window_position(
    config_manager: &Arc<ConfigManager>,
    host_api: &Arc<HostApi>,
    app_handle: &tauri::AppHandle,
) -> bool {
    let wake_on_fullscreen = config_manager
        .get_component_setting("window-behavior", "is_wake_on_fullscreen")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !wake_on_fullscreen && crate::utils::windows::is_foreground_fullscreen() {
        return false;
    }

    // 读取窗口定位配置
    let enable_drag = config_manager
        .get_component_setting("window-behavior", "is_enable_drag_window")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let follow_mouse = config_manager
        .get_component_setting("window-behavior", "show_pos_follow_mouse")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let vertical_ratio = config_manager
        .get_component_setting("appearance", "vertical_position_ratio")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.28);
    let window_width = config_manager
        .get_component_setting("appearance", "window_width")
        .and_then(|v| v.as_f64())
        .unwrap_or(800.0) as i32;

    // 读取拖拽模式保存的位置
    let saved_position = if enable_drag {
        let x = config_manager
            .get_component_setting("window-behavior", "window_position_x")
            .and_then(|v| v.as_f64())
            .map(|v| v as i32)
            .unwrap_or(0);
        let y = config_manager
            .get_component_setting("window-behavior", "window_position_y")
            .and_then(|v| v.as_f64())
            .map(|v| v as i32)
            .unwrap_or(0);
        if x != 0 || y != 0 {
            Some(WindowPosition { x, y })
        } else {
            None
        }
    } else {
        None
    };

    // 收集显示器信息并计算窗口位置
    let monitors = collect_monitor_info(app_handle);
    let request = PositionRequest {
        enable_drag_window: enable_drag,
        saved_position,
        follow_mouse,
        vertical_position_ratio: vertical_ratio,
        window_width,
        monitors,
    };

    if let Ok(pos) = host_api.compute_window_position(request).await {
        host_api.set_window_position(pos);
    }

    true
}

/// 若拖拽模式已启用，将当前窗口位置持久化到 ConfigManager。
fn save_window_position_if_drag(
    config_manager: &Arc<ConfigManager>,
    app_handle: &tauri::AppHandle,
) {
    let enable_drag = config_manager
        .get_component_setting("window-behavior", "is_enable_drag_window")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !enable_drag {
        return;
    }
    if let Some(window) = app_handle.get_webview_window("main") {
        if let Ok(pos) = window.outer_position() {
            let mut current = config_manager
                .get_settings("window-behavior")
                .unwrap_or_else(|| serde_json::json!({}));
            if let Some(obj) = current.as_object_mut() {
                obj.insert("window_position_x".to_string(), serde_json::json!(pos.x));
                obj.insert("window_position_y".to_string(), serde_json::json!(pos.y));
            }
            if let Err(e) = config_manager.apply_settings("window-behavior", current) {
                warn!("[save_window_position] 持久化窗口位置失败: {}", e);
            }
        }
    }
}

/// 初始化搜索栏窗口。
///
/// 注册焦点丢失回调（隐藏窗口 + 重置会话）。
/// 窗口位置由后端热键回调在 show_window() 前统一计算和设置。
///
/// 窗口失焦检测由 FocusMonitor（push-based）统一管理，
/// 本函数仅负责注册业务层回调，不直接处理窗口事件。
fn init_search_bar_window(app: &mut App) {
    let state = app.state::<Arc<AppState>>();
    let host_api = state.get_host_api();
    let session_router = state.get_session_router().clone();
    let config_manager = state.get_config_manager();
    let app_handle = state.get_main_handle();

    // 注册焦点丢失回调：保存拖拽位置 → 隐藏窗口 → 重置会话 → 通知前端
    let core_handle = state.get_core_handle();
    let host_api_for_cb = host_api.clone();
    let config_manager_for_cb = config_manager.clone();
    let app_handle_for_cb = app_handle.clone();
    core_handle.register_focus_callback(
        "main_window_focus",
        Arc::new(move |_event| {
            let host_api = host_api_for_cb.clone();
            let session_router = session_router.clone();
            let config_manager = config_manager_for_cb.clone();
            let app_handle = app_handle_for_cb.clone();
            tauri::async_runtime::spawn(async move {
                save_window_position_if_drag(&config_manager, &app_handle);
                host_api.hide_window().await;

                let reset_plugins = config_manager
                    .get_component_setting("general", "reset_session_on_wake")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                if session_router.reset_session(reset_plugins) {
                    let _ = app_handle.emit("session-reset", ());
                }
            });
        }),
    );
}

/// 从 Tauri AppHandle 收集可用显示器信息，供窗口定位使用。
fn collect_monitor_info(app_handle: &tauri::AppHandle) -> Vec<MonitorInfo> {
    app_handle
        .get_webview_window("main")
        .and_then(|w| w.available_monitors().ok())
        .map(|monitors| {
            monitors
                .iter()
                .map(|m| {
                    let pos = m.position();
                    let size = m.size();
                    MonitorInfo {
                        x: pos.x,
                        y: pos.y,
                        width: size.width,
                        height: size.height,
                        scale_factor: m.scale_factor(),
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

fn init_setting_window(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let setting_window = Arc::new(
            tauri::WebviewWindowBuilder::new(
                &app,
                "setting_window",
                WebviewUrl::App("/setting_window.html".into()),
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
