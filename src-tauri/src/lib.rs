pub mod bootstrap;
pub mod builtin_plugin;
pub mod cli_server;
pub mod commands;
pub mod core;
pub mod logging;
pub mod plugin_framework;
pub mod sdk;
pub mod state;
pub mod tray;
pub mod utils;
pub mod window;

use crate::logging::{init_logging, log_application_shutdown, log_application_start};
use crate::sdk::HostApi;
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
use tracing::{debug, info, warn};
use zerolaunch_platform_windows::windows_capabilities;
use zerolaunch_platform_windows::ComGuard;
use zerolaunch_platform_windows::WindowsAppEnumerator;
use zerolaunch_platform_windows::WindowsAppLauncher;
use zerolaunch_platform_windows::WindowsAutoStartManager;
use zerolaunch_platform_windows::WindowsClipboardProvider;
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
use zerolaunch_plugin_api::services::path::KnownPath;
use zerolaunch_plugin_api::services::storage::storage_service::StorageService;
use zerolaunch_plugin_api::services::timer::TokioTimerManager;
use zerolaunch_plugin_api::services::AppResourceService;
use zerolaunch_plugin_api::services::PathResolver;
static IS_EXITING: AtomicBool = AtomicBool::new(false);

pub async fn do_cleanup_before_exit(state: Arc<AppState>) {
    info!("执行退出前清理工作...");
    let config_manager = state.get_config_manager();
    if let Err(e) = config_manager.save_to_storage() {
        warn!("退出前配置保存失败: {}", e);
    }
    // 退出前同步到远程存储
    let host_api = state.get_host_api();
    crate::bootstrap::sync_config_to_remote(&config_manager, &host_api).await;
    // 注销全局快捷键和双击 Ctrl 监听器
    if let Err(e) = host_api.unregister_all_hotkeys().await {
        warn!("退出前注销快捷键失败: {:?}", e);
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
        .register_uri_scheme_protocol("zlplugin", move |app, request| {
            let uri = request.uri().to_string();
            let state = app.app_handle().state::<Arc<AppState>>();
            let pm = state.get_plugin_manager();
            match pm.handle_zlplugin_uri(&uri) {
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
                bootstrap::init_app_state(
                    app,
                    path_resolver,
                    app_data_dir,
                    icon_cache_dir,
                    config_dir,
                )
                .await;

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

                let state_for_deeplink = app.state::<Arc<AppState>>().inner().clone();
                app.deep_link().on_open_url(move |event| {
                    let urls = event.urls();
                    debug!("收到深度链接事件: {:?}", urls);
                    let state = state_for_deeplink.clone();
                    tauri::async_runtime::spawn(async move {
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
            crate::commands::config_file::config_get_version,
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
            crate::commands::debug::debug_simulate_query,
            // Debug Tools
            crate::commands::debug::debug_test_search_time,
            crate::commands::debug::debug_test_index_time,
            crate::commands::debug::debug_get_search_keys,
            crate::commands::debug::debug_search_detail,
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
                let state = app_handle.state::<Arc<AppState>>().inner().clone();
                tauri::async_runtime::spawn(async move {
                    do_cleanup_before_exit(state).await;
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
                crate::window::save_window_position_if_drag(&config_manager, &app_handle);
                host_api.hide_window().await;

                let reset_plugins = config_manager
                    .get_component_setting("general-config", "reset_session_on_wake")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                if session_router.reset_session(reset_plugins) {
                    let _ = app_handle.emit("session-reset", ());
                }
            });
        }),
    );
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
