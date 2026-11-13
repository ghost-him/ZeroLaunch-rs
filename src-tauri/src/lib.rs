pub mod bootstrap;
pub mod builtin_plugin;
pub mod cli_server;
pub mod commands;
pub mod core;
pub mod kernel;
pub mod logging;
pub mod plugin_framework;
pub mod plugin_market;
pub mod runtime;
pub mod sdk;
pub mod state;
pub mod tray;
pub mod updater;
pub mod utils;
pub mod window;

use crate::logging::{init_logging, log_application_start};
use crate::state::app_state::AppState;
use crate::window::save_window_position_if_drag;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::App;
use tauri::LogicalSize;
use tauri::Manager;
use tauri::WebviewUrl;
use tauri_plugin_deep_link::DeepLinkExt;
use tracing::{debug, info};
use zerolaunch_platform_windows::ComGuard;
use zerolaunch_platform_windows::WindowsPathResolver;
use zerolaunch_platform_windows::{
    acquire_single_instance, SecondaryInstanceCallback, SingleInstanceStatus,
};
use zerolaunch_plugin_api::services::focus_monitor::{FocusCallback, FocusEvent};
use zerolaunch_plugin_api::services::path::path_resolver::PathResolver;
use zerolaunch_plugin_api::services::path::KnownPath;
static IS_EXITING: AtomicBool = AtomicBool::new(false);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 进程级 runtime：Tauri 外壳、CLI 与内核共用同一 handle。
    tauri::async_runtime::set(crate::runtime::handle());
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
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(Arc::new(AppState::new()))
        .setup(move |app| {
            // 单实例校验：后续实例通知首个实例后直接退出，避免重复初始化。
            let single_instance_callback: SecondaryInstanceCallback = Arc::new({
                let app_handle = app.handle().clone();
                move |_args, _cwd| {
                    crate::utils::notification::notify(
                        &app_handle,
                        "ZeroLaunch",
                        "程序已在运行中，无需重复启动",
                    );
                }
            });
            if acquire_single_instance(&app.config().identifier, single_instance_callback)
                == SingleInstanceStatus::Secondary
            {
                std::process::exit(0);
            }

            let app_data_dir = path_resolver.resolve_path(KnownPath::AppDataDir).unwrap();
            let icon_cache_dir = path_resolver
                .resolve_path(KnownPath::AppIconCacheDir)
                .unwrap();
            let config_dir = path_resolver.resolve_path(KnownPath::AppConfigDir).unwrap();
            let state_for_theme = app.state::<Arc<AppState>>().inner().clone();

            info!("应用数据目录: {}", app_data_dir);
            info!("图标缓存目录: {}", icon_cache_dir);
            info!("配置目录: {}", config_dir);

            crate::runtime::block_on(async move {
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

                info!("=== 系统托盘已由 TauriHost 初始化 ===");

                info!("正在注册深度链接");
                app.deep_link().register_all().expect("无法注册深度链接");
                info!("深度链接注册成功");

                let state_for_deeplink = app.state::<Arc<AppState>>().inner().clone();
                app.deep_link().on_open_url(move |event| {
                    let urls = event.urls();
                    debug!("收到深度链接事件: {:?}", urls);
                    let state = state_for_deeplink.clone();
                    crate::runtime::spawn(async move {
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

            // 系统主题监控：平台实现见 platform-windows，宿主注册回调推送前端事件
            bootstrap::init_system_theme_monitor(state_for_theme.get_host());

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Bridge: 搜索与会话管理
            crate::commands::bridge::bridge_query,
            crate::commands::bridge::bridge_confirm,
            crate::commands::bridge::bridge_wake,
            crate::commands::bridge::bridge_wake_plugin,
            crate::commands::bridge::bridge_refresh_candidates,
            crate::commands::bridge::bridge_exclude_options,
            crate::commands::bridge::bridge_exclude_candidate,
            crate::commands::bridge::bridge_hide_window,
            crate::commands::bridge::bridge_get_system_theme,
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
            // Debug Tools
            crate::commands::debug::debug_test_search_time,
            crate::commands::debug::debug_test_index_time,
            crate::commands::debug::debug_get_search_keys,
            crate::commands::debug::debug_search_detail,
            crate::commands::debug::debug_copy_search_detail,
            crate::commands::debug::debug_get_agent_traces,
            // 剪贴板
            crate::commands::clipboard::copy_to_clipboard,
            // Third-party Plugin Management
            crate::commands::plugin::plugin_list,
            crate::commands::plugin::plugin_get_manifest,
            crate::commands::plugin::plugin_get_detail,
            crate::commands::plugin::plugin_install_local,
            crate::commands::plugin::plugin_inspect_package,
            crate::commands::plugin::plugin_reload,
            crate::commands::plugin::plugin_uninstall,
            crate::commands::plugin::plugin_set_enabled,
            crate::commands::plugin::plugin_get_logs,
            // Plugin Market
            crate::commands::plugin_market::market_list,
            crate::commands::plugin_market::market_get_meta,
            crate::commands::plugin_market::market_install,
            crate::commands::plugin_market::market_preview_package,
            crate::commands::plugin_market::market_discard_preview,
            crate::commands::cli::cli_get_info,
            // i18n
            crate::commands::i18n::i18n_get_plugin_translations,
            // AI 助手
            crate::commands::help::help_search,
            crate::commands::help::help_ask,
            // 路径类型查询（设置页拖放判定文件/目录）
            crate::commands::path::path_kind,
            // 日志导出（打包当前日志目录为 zip）
            crate::commands::log_export::export_logs,
            // 版本更新检查
            crate::commands::update::update_check,
            crate::commands::update::update_get_status,
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
                crate::runtime::spawn(async move {
                    bootstrap::do_cleanup_before_exit(state).await;
                    info!("清理完成，正在退出程序...");
                    app_handle.exit(0);
                });
            }
            _ => {}
        });
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
    let session_dispatcher = state.get_session_dispatcher().clone();
    let config_manager = state.get_config_manager();

    // 注册焦点丢失回调：保存拖拽位置 → 隐藏窗口 → 重置会话（Dispatcher 内部推送会话事件）
    let core_handle = state.get_core_handle();
    let host_api_for_cb = host_api.clone();
    let config_manager_for_cb = config_manager.clone();
    let host_for_cb = state.get_host();
    let on_focus_lost: FocusCallback = Arc::new(move |_event: FocusEvent| {
        let host_api = host_api_for_cb.clone();
        let session_dispatcher = session_dispatcher.clone();
        let config_manager = config_manager_for_cb.clone();
        let host = host_for_cb.clone();
        crate::runtime::spawn(async move {
            save_window_position_if_drag(&config_manager, &host).await;
            host_api.hide_window().await;

            let reset_plugins = config_manager
                .get_component_setting("general-config", "reset_session_on_wake")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            // 会话重置由 Dispatcher 内部推送 session-state（kind: .host., view: .none.），
            // 原 session-reset 事件已删除（统一会话事件通道，见设计 §4.4）。
            session_dispatcher.reset_session(reset_plugins);
        });
    });
    core_handle.register_focus_callback("main_window_focus", on_focus_lost.clone());

    // 关闭主窗口（Alt+F4）等价于失焦：阻止销毁，复用同一隐藏与重置流程。
    if let Some(window) = app.get_webview_window("main") {
        window.on_window_event(move |event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                on_focus_lost(FocusEvent::Lost);
            }
        });
    }
}

fn init_setting_window(app: tauri::AppHandle) {
    // 标题由界面语言决定（与托盘菜单共用后端译表）；语言切换时由 bootstrap 刷新
    let title = bootstrap::setting_window_title(app.state::<Arc<AppState>>().inner());
    crate::runtime::spawn(async move {
        let setting_window = Arc::new(
            tauri::WebviewWindowBuilder::new(
                &app,
                "setting_window",
                WebviewUrl::App("/setting_window.html".into()),
            )
            .title(title)
            .visible(false)
            // 开启拖放以支持「第三方插件管理」页拖拽安装插件（webview 级 onDragDropEvent 接收真实路径）
            .drag_and_drop(true)
            .build()
            .expect("无法创建设置窗口"),
        );
        setting_window
            .set_size(LogicalSize::new(1150, 650))
            .expect("无法设置设置窗口大小");
        // 关闭按钮不在后端直接隐藏：前端要先确认未保存的修改（或直接隐藏）。
        // 这里只阻止窗口销毁，隐藏动作由前端收到 close-requested 后执行。
        setting_window.on_window_event(|event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
            }
        });
    });
}
