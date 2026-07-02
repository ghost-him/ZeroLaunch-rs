//! 应用初始化序列。
//!
//! 从 lib.rs 提取的核心初始化函数，负责：
//! - `init_app_state` — 创建 HostApi、ConfigManager、PluginManager 并编排初始化顺序
//! - `init_plugin_system` — inventory 自动发现、管道构建、事件订阅

use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use tauri::{App, Emitter, Manager};
use tracing::{debug, info, warn};
use zerolaunch_platform_windows::WindowsFocusMonitor;
use zerolaunch_platform_windows::WindowsHotkeyManager;
use zerolaunch_plugin_api::services::hotkey::types::HotkeyEventFilter;
use zerolaunch_plugin_api::services::storage::local_storage::LocalStorageService;
use zerolaunch_plugin_api::services::storage::storage_service::StorageService;
use zerolaunch_plugin_api::services::AppResourceService;

use crate::core::config::event::create_plugin_event_bus;
use crate::core::config::{ConfigEvent, ConfigManager};
use crate::plugin_framework::manager::PluginManager;
use crate::plugin_framework::CandidatePipeline;
use crate::state::app_state::AppState;
use crate::tray::TrayManager;
use crate::window::{prepare_window_position, save_window_position_if_drag};

/// 将当前配置序列化并同步到远程存储后端（fire-and-forget）。
///
/// 从 ConfigManager 构建 PersistentConfig，序列化为 JSON 字节，
/// 通过 HostApi 的 StorageService 上传。失败仅记日志，不阻断。
pub(crate) async fn sync_config_to_remote(
    config_manager: &ConfigManager,
    host_api: &crate::sdk::HostApi,
) {
    let config = config_manager.build_persistent_config();
    let json_bytes = match serde_json::to_vec(&config) {
        Ok(bytes) => bytes,
        Err(e) => {
            tracing::warn!("配置序列化失败，跳过远程同步: {}", e);
            return;
        }
    };
    let storage = host_api.storage();
    if let Err(e) = storage.upload("zerolaunch_config.json", &json_bytes).await {
        tracing::warn!("配置远程同步失败: {}", e);
    }
}

/// 初始化应用状态（HostApi、ConfigManager、PluginManager）。
///
/// 调用方（lib.rs 的 `run()`）将 `init_app_state` 置于 `setup` 闭包的
/// `tauri::async_runtime::block_on` 中执行。
pub(crate) async fn init_app_state(
    app: &mut App,
    path_resolver: Arc<zerolaunch_platform_windows::WindowsPathResolver>,
    app_data_dir: String,
    icon_cache_dir: String,
    config_dir: String,
) {
    debug!("开始初始化应用状态");

    let state: Arc<AppState> = app.state::<Arc<AppState>>().inner().clone();

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
    let app_handle_for_third_party_plugins = app_handle.clone();

    let host_api = Arc::new(
        crate::build_windows_host_api_builder(
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

    // 将核心程序对于平台的操作也视为是一个插件，共用同一套pluginhandle
    let core_handle = host_api.register("core", Default::default());
    state.set_core_handle(core_handle.clone());
    info!("Core PluginHandle 注册完成");

    #[cfg(feature = "inspector")]
    {
        use crate::plugin_framework::inspector::Inspector;
        state.set_inspector(Arc::new(Inspector::new(200)));
        info!("Plugin Inspector 已启用 (容量: 200)");
    }

    let tray_manager = Arc::new(TrayManager::new(host_api.clone()));
    state.set_tray_manager(tray_manager);
    info!("TrayManager 创建完成");

    info!("=== Phase 2: Core 初始化 - 创建 ConfigManager ===");

    let config_manager = Arc::new(ConfigManager::new(PathBuf::from(&config_dir)));
    info!("ConfigManager 初始化完成");

    info!("=== Phase 3: PluginManager 初始化 ===");

    // 创建 PluginRuntimeEvent 通道（PM → CM 解耦管道）。
    // 接收端在 init_plugin_system 中通过 subscribe() 创建，与 ConfigEvent 模式一致。
    let (plugin_event_tx, _plugin_event_rx) = create_plugin_event_bus(256);

    // 创建 PluginManager（通过 PluginRuntimeEvent 广播通道与 CM 通信，不再直接依赖 CM）
    let plugin_manager = Arc::new(PluginManager::new());
    plugin_manager.set_plugin_event_tx(plugin_event_tx);
    plugin_manager.set_host_api(host_api.clone());
    state.set_plugin_manager(plugin_manager.clone());

    // 将 config_manager 保存到 AppState（必须在 PluginManager 之后，因为 clone 语义）
    state.set_config_manager(config_manager);

    // 初始化内置 + 第三方插件
    init_plugin_system(&state).await;

    info!("=== Phase 4: 第三方插件加载 ===");

    plugin_manager.init_host_manager(Path::new(&app_data_dir));
    plugin_manager
        .load_all_third_party(app_handle_for_third_party_plugins)
        .await;

    // 批量加载后刷新候选项缓存，确保第三方插件的数据源被纳入。
    // 各插件的 PluginRegistered 事件也会触发独立 refresh，但批量场景下
    // 可能存在事件尚未处理完的竞态，此处作为最终兜底保证缓存完整。
    state.get_session_router().refresh_candidates().await;

    // Start CLI HTTP server
    info!("=== Phase 5: 启动 CLI HTTP 服务器... ===");
    let cli_handle =
        crate::cli_server::server::start(state.clone(), &PathBuf::from(&app_data_dir)).await;
    match cli_handle {
        Ok(handle) => info!("CLI HTTP 服务器已启动于 127.0.0.1:{}", handle.port),
        Err(e) => tracing::warn!("CLI HTTP 服务器启动失败: {}", e),
    }

    info!(
        "应用状态初始化完成 (HostApi, ConfigManager, {} 个已注册组件)",
        state.get_config_manager().get_all_components().len()
    );
}

/// 初始化插件系统。
///
/// 核心流程：
/// - Phase A: inventory 自动发现并注册所有内置组件 + 快捷键回调
/// - Phase B: 加载持久化配置
/// - Phase C: 构建候选项管道和搜索管道
pub(crate) async fn init_plugin_system(state: &Arc<AppState>) {
    let session_router = state.get_session_router();
    let config_manager = state.get_config_manager();
    let plugin_manager = state.get_plugin_manager();

    session_router.set_config_manager(config_manager.clone());

    // 订阅配置事件
    let event_router = session_router.clone();
    let app_handle = state.get_main_handle();
    let cm_for_events = config_manager.clone();
    let host_api_for_events = state.get_host_api();
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
                    // 配置变更后自动触发远程同步（fire-and-forget）
                    match &event {
                        ConfigEvent::SettingsChanged { .. }
                        | ConfigEvent::EnabledChanged { .. } => {
                            sync_config_to_remote(&cm_for_events, &host_api_for_events).await;
                        }
                        _ => {}
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

    // 订阅 PluginRuntimeEvent（PM → CM 解耦管道）
    let cm_listener = config_manager.clone();
    let mut plugin_event_rx = plugin_manager.plugin_event_tx().subscribe();
    tauri::async_runtime::spawn(async move {
        loop {
            match plugin_event_rx.recv().await {
                Ok(event) => cm_listener.handle_plugin_event(&event),
                Err(tokio::sync::broadcast::error::RecvError::Lagged(count)) => {
                    warn!("PluginRuntimeEvent 接收器落后 {} 条消息", count);
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    info!("PluginRuntimeEvent 通道已关闭，退出监听");
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

    let collected = plugin_manager.init_builtins();

    collected.for_each_configurable(|c| {
        config_manager.register(c.clone());
    });

    // 注册内置运行时组件到 SessionRouter
    for (_, ex) in &collected.executors {
        session_router.register_executor(ex.clone());
    }
    for (_, se) in &collected.search_engines {
        session_router.register_search_engine(se.clone());
    }
    for (_, sb) in &collected.score_boosters {
        session_router.register_score_booster(sb.clone());
    }
    for (_, p) in &collected.plugins {
        session_router.plugin_service().register(p.clone());
    }

    info!(
        "Phase A 完成: 共注册 {} 个组件（其中内置 {} 个）",
        config_manager.get_all_components().len(),
        plugin_manager.list_builtins().len(),
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
    if let Err(e) = config_manager.load_from_storage() {
        warn!("加载持久化配置失败: {}", e);
    }

    // ========================================================================
    // Phase C: 构建管道
    // ========================================================================
    info!("=== Phase C: 构建业务管道 ===");

    info!("构建候选管道...");
    let mut candidate_pipeline = CandidatePipeline::new();
    for (_, ds) in &collected.data_sources {
        candidate_pipeline.add_source(ds.clone());
    }
    for (_, ko) in &collected.keyword_optimizers {
        candidate_pipeline.add_keyword_optimizer(ko.clone());
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
