//! 应用初始化序列。
//!
//! 从 lib.rs 提取的核心初始化函数，负责：
//! - `init_app_state` — 创建 HostApi、ConfigManager、PluginManager 并编排初始化顺序
//! - `init_plugin_system` — inventory 自动发现、管道构建、事件订阅

use crate::builtin_plugin::config::auto_refresh_config::AutoRefreshSettings;
use crate::builtin_plugin::config::hotkey_config::{settings_to_hotkey_config, HotkeySettings};
use crate::platform::{PlatformFocusMonitor, PlatformHotkeyManager, PlatformPathResolver};
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{App, Emitter, Manager};
use tracing::{debug, info, warn};
use zerolaunch_plugin_api::host::PluginSdkConfig;
use zerolaunch_plugin_api::services::hotkey::types::HotkeyEventFilter;
use zerolaunch_plugin_api::services::installation_monitor::InstallationEventKind;
use zerolaunch_plugin_api::services::storage::local_storage::LocalStorageService;
use zerolaunch_plugin_api::services::storage::storage_service::StorageService;
use zerolaunch_plugin_api::services::AppResourceService;
use zerolaunch_plugin_api::PluginContext;

use crate::core::app_command;
use crate::core::config::bias_settings::{bias_settings_to_rules, BiasSettings};
use crate::core::config::event::create_plugin_event_bus;
use crate::core::config::{ConfigEvent, ConfigManager};
use crate::core::i18n::I18nManager;
use crate::plugin_framework::inspector::Inspector;
use crate::plugin_framework::manager::PluginManager;
use crate::plugin_framework::plugin_wake_executor::PluginWakeExecutor;
use crate::sdk::HostApi;
use crate::state::app_state::AppState;
use crate::tray::TrayManager;
use crate::utils::trace_id::generate_trace_id;
use crate::window::{prepare_window_position, save_window_position_if_drag};

/// 启动定时自动刷新任务。
///
/// 每分钟醒来一次，读取 auto-refresh-config 的间隔配置（分钟，0=禁用）；
/// 距上次刷新（SessionDispatcher.last_refresh，所有触发源共享的单一时间基准）
/// 达到间隔时触发 refresh_candidates —— 监控/手动/配置联动刷新后自动重置基准，
/// 天然避免短时间重复刷新，无需老版 RefreshScheduler 的调度线程与条件变量。
fn spawn_auto_refresh_task(state: Arc<AppState>) {
    tauri::async_runtime::spawn(async move {
        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            ticker.tick().await;
            // 读取定时刷新配置；组件缺失或解析失败按禁用处理
            let interval_mins = state
                .get_config_manager()
                .get_settings("auto-refresh-config")
                .and_then(|v| serde_json::from_value::<AutoRefreshSettings>(v).ok())
                .map(|s| s.auto_refresh_interval_mins)
                .unwrap_or(0.0);
            if interval_mins <= 0.0 {
                continue;
            }
            let dispatcher = state.get_session_dispatcher();
            let interval = std::time::Duration::from_secs_f64(interval_mins * 60.0);
            if dispatcher.last_refresh_elapsed() >= interval {
                dispatcher.refresh_candidates().await;
                info!(
                    "定时刷新完成，共 {} 个候选项",
                    dispatcher.get_cached_candidates_count()
                );
            }
        }
    });
}

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
    path_resolver: Arc<PlatformPathResolver>,
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

    // ModelManager 由 AppState 持有，HostApi 与 core handle 共享同一实例。
    let model_manager = state.get_model_manager();

    let host_api = Arc::new(
        crate::build_platform_host_api_builder(
            icon_cache_dir,
            default_app_icon_path,
            default_web_icon_path,
            path_resolver,
            default_storage,
            app_resource,
        )
        .hotkey_manager(Arc::new(PlatformHotkeyManager::new(app_handle)))
        .focus_monitor(Arc::new(PlatformFocusMonitor::new(
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
        .model_service(model_manager.clone())
        .build()
        .expect("Failed to build HostApi"),
    );
    state.set_host_api(host_api.clone());
    info!("HostApi 初始化完成");

    // 预载图标文件缓存到内存（L1），消除冷启动后首次查询的磁盘读；
    // 后台执行不阻塞后续初始化，查询路径 L1 未命中时仍回退按需读 L2。
    let host_api_for_preload = host_api.clone();
    tauri::async_runtime::spawn(async move {
        host_api_for_preload.preload_icon_cache().await;
    });

    // 将核心程序对于平台的操作也视为是一个插件，共用同一套pluginhandle
    let core_handle = host_api.register("core", Default::default());
    state.set_core_handle(core_handle.clone());
    info!("Core PluginHandle 注册完成");

    // embedding 缓存经 core PluginHandle 的本地缓存空间挂载：
    // 路径 <app_data>/plugin-cache/core/model-embedding/，不经 StorageService，
    // WebDAV 同步模式不会上传远端。
    let embedding_cache = Arc::new(crate::core::model::EmbeddingCache::new(core_handle.clone()));
    model_manager.set_cache(embedding_cache);

    // 注册安装监控刷新回调：开始菜单变化（平台层去抖合并后）自动刷新候选项缓存。
    // 必须在 init_plugin_system 加载持久化配置（可能触发监控启动）之前注册，
    // 否则监控启动后的事件窗口期内回调缺失，事件到达无人处理。
    let state_for_install_callback = state.clone();
    let install_event_handle = state.get_main_handle();
    core_handle.register_installation_callback(
        "host:refresh_candidates",
        Arc::new(move |event| {
            let state = state_for_install_callback.clone();
            let app_handle = install_event_handle.clone();
            tauri::async_runtime::spawn(async move {
                let dispatcher = state.get_session_dispatcher();
                dispatcher.refresh_candidates().await;
                info!(
                    "安装监控事件（{:?}，{} 个路径）触发自动刷新，共 {} 个候选项",
                    event.kind,
                    event.changed_paths.len(),
                    dispatcher.get_cached_candidates_count()
                );
                // 推送安装事件给前端：形状与后端 InstallationEvent 对齐
                // （kind + changedPaths），前端按需自行判断，不做启发式映射。
                let kind_str = match event.kind {
                    InstallationEventKind::Created => "created",
                    InstallationEventKind::Modified => "modified",
                    InstallationEventKind::Removed => "removed",
                    InstallationEventKind::Other => "other",
                };
                let _ = app_handle.emit(
                    "installation-event",
                    serde_json::json!({
                        "kind": kind_str,
                        "changedPaths": event.changed_paths,
                    }),
                );
            });
        }),
    );
    info!("安装监控刷新回调已注册");

    state.set_inspector(Arc::new(Inspector::new(200)));
    info!("Plugin Inspector 已创建 (容量: 200，录制默认关闭)");

    // 创建 AppCommand 通道并初始化全局发送端。
    // 命令通道是应用基础设施（有且仅有一个消费者），使用全局 OnceLock 而非依赖注入——
    // 避免将通道穿过 PluginManager、InventoryContext 等不消费它的中间结构体。
    // 详见 core/app_command.rs 顶部注释。
    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel::<app_command::AppCommand>(32);
    app_command::init_command_channel(cmd_tx);

    // 后端翻译服务：读取打包进资源的语言包（vite 构建时从 src-ui/i18n/locales 复制）。
    let i18n_manager = I18nManager::load(resource_dir.join("locales"));
    state.set_i18n_manager(i18n_manager.clone());
    info!("I18nManager 初始化完成");

    let tray_manager = Arc::new(TrayManager::new(host_api.clone(), i18n_manager));
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
    plugin_manager.set_i18n_manager(state.get_i18n_manager());
    state.set_plugin_manager(plugin_manager.clone());

    // 将 config_manager 保存到 AppState（必须在 PluginManager 之后，因为 clone 语义）
    state.set_config_manager(config_manager);
    // 初始化内置 + 第三方插件（返回内置组件 id 集合，供 init_host_manager 注入冲突预检）
    let builtin_component_ids = init_plugin_system(&state).await;
    info!("Phase 3 完成: 插件系统初始化就绪");

    info!("=== Phase 4: 第三方插件加载 ===");

    plugin_manager.init_host_manager(Path::new(&app_data_dir), builtin_component_ids);
    plugin_manager
        .load_all_third_party(app_handle_for_third_party_plugins)
        .await;

    // 批量加载后刷新候选项缓存，确保第三方插件的数据源被纳入。
    // 各插件的 PluginRegistered 事件也会触发独立 refresh，但批量场景下
    // 可能存在事件尚未处理完的竞态，此处作为最终兜底保证缓存完整。
    state.get_session_dispatcher().refresh_candidates().await;
    info!(
        "Phase 4 完成: 第三方插件加载完成，共 {} 个候选项",
        state.get_session_dispatcher().get_cached_candidates_count()
    );

    // Start CLI HTTP server
    info!("=== Phase 5: 启动 CLI HTTP 服务器... ===");
    let cli_handle =
        crate::cli_server::server::start(state.clone(), &PathBuf::from(&app_data_dir)).await;
    match cli_handle {
        Ok(handle) => info!("CLI HTTP 服务器已启动于 127.0.0.1:{}", handle.port),
        Err(e) => tracing::warn!("CLI HTTP 服务器启动失败: {}", e),
    }

    // 启动 AppCommand 消费者 task
    info!("=== Phase 6: 启动 AppCommand 消费者 ===");
    spawn_app_command_consumer(cmd_rx, state.clone());
    info!("Phase 6 完成: AppCommand 消费者已启动");

    // 启动定时自动刷新任务（兜底：安装监控关闭或漏掉的变化时，索引仍会定期更新）
    spawn_auto_refresh_task(state.clone());
    info!("定时自动刷新任务已启动");

    info!(
        "应用状态初始化完成 (HostApi, ConfigManager, {} 个已注册组件)",
        state.get_config_manager().get_all_components().len()
    );
}

/// 将 general-config 的持久化语言同步到 I18nManager 并重建托盘菜单。
/// 调用点：持久化配置加载完成后、general-config 语言变更事件处理时。
fn sync_backend_language(state: &Arc<AppState>, config_manager: &ConfigManager) {
    let i18n = state.get_i18n_manager();
    let lang = config_manager.get_settings("general-config").and_then(|v| {
        v.get("language")
            .and_then(|x| x.as_str().map(str::to_string))
    });
    // 语言未变化时不重建托盘菜单：general-config 的任意设置变更
    // （自动启动、日志级别等，与语言无关）都会触发本函数，
    // 全量原生菜单重建仅在语言真正切换时执行。
    if let Some(lang) = lang {
        if lang != i18n.current_language() {
            i18n.set_language(&lang);
            if let Some(tray) = state.get_tray_manager() {
                tray.update_menu_language();
            }
        }
    }
}

/// 将外观配置中的主题模式同步到 HostApi，供插件查询实际主题。
fn sync_backend_theme(host_api: &HostApi, config_manager: &ConfigManager) {
    let theme = config_manager
        .get_settings("appearance-config")
        .and_then(|s| s.get("theme").and_then(|v| v.as_str()).map(str::to_string));
    if let Some(theme) = theme {
        host_api.set_theme_mode(&theme);
    }
}

/// 初始化插件系统。
///
/// 核心流程：
/// - Phase A: inventory 自动发现并注册所有内置组件 + 快捷键回调
/// - Phase B: 加载持久化配置
/// - Phase C: 构建候选项管道和搜索管道
pub(crate) async fn init_plugin_system(state: &Arc<AppState>) -> HashSet<String> {
    let session_dispatcher = state.get_session_dispatcher();
    let config_manager = state.get_config_manager();
    let plugin_manager = state.get_plugin_manager();

    session_dispatcher.set_config_manager(config_manager.clone());
    session_dispatcher.set_i18n_manager(state.get_i18n_manager());

    // 注入会话状态推送回调：路由确定插件后立即推送，不等慢查询响应，
    // 保证慢查询期间的新输入也能正确应用防抖（原 panel-interaction 无条件推送不变式，
    // 事件统一为 session-state）。
    let app_handle_for_policy = state.get_main_handle();
    let policy_emitter = Arc::new(move |event| {
        let _ = app_handle_for_policy.emit("session-state", event);
    });
    session_dispatcher.set_session_emitter(policy_emitter);

    // 订阅配置事件
    let event_router = session_dispatcher.clone();
    let app_handle = state.get_main_handle();
    let cm_for_events = config_manager.clone();
    let host_api_for_events = state.get_host_api();
    let state_for_events = state.clone();
    let model_manager_for_events = state.get_model_manager();
    let mut event_receiver = config_manager.event_sender().subscribe();
    tauri::async_runtime::spawn(async move {
        loop {
            match event_receiver.recv().await {
                Ok(event) => {
                    event_router.handle_config_event(&event).await;
                    // 内置模型配置变更时重建提供方并刷新模型清单
                    model_manager_for_events
                        .handle_config_event(&cm_for_events, &event)
                        .await;
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
                        // 外观配置变更时同步主题模式到 HostApi（插件 get_theme 查询）
                        if component_id == "appearance-config" {
                            sync_backend_theme(&host_api_for_events, &cm_for_events);
                        }
                        // 语言切换时同步后端翻译服务并重建托盘菜单（即时生效）
                        if component_id == "general-config" {
                            sync_backend_language(&state_for_events, &cm_for_events);
                        }
                        // 会话投影随配置变更重新推送（如面板内调整防抖延迟）
                        event_router.reemit_current_session();
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
                Ok(event) => cm_listener.handle_plugin_event(&event).await,
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
    session_dispatcher.set_host_api(host_api.clone());
    info!("事件订阅循环已启动（ConfigEvent + PluginRuntimeEvent）");

    // ========================================================================
    // Phase A: inventory 自动发现并注册所有内置组件
    // ========================================================================
    info!("=== Phase A: inventory 自动发现并注册所有内置组件 ===");

    let collected = plugin_manager.init_builtins(session_dispatcher.clone());

    for c in collected.configurables() {
        config_manager.register(c.clone()).await;
    }

    // 注册内置运行时组件到 PluginComponentRegistry 和 SessionDispatcher
    for (c, ex) in &collected.executors {
        if config_manager.find_configurable(c.component_id()).is_some() {
            session_dispatcher.register_executor(ex.clone());
        } else {
            warn!(
                "组件 {} 的 Configurable 注册失败，跳过 Executor 注册",
                c.component_id()
            );
        }
    }
    // 宿主内置执行器（不随 inventory 注册）：沉浸式插件候选项唤醒 → wake_plugin。
    session_dispatcher.register_executor(Arc::new(PluginWakeExecutor::new(Arc::downgrade(
        session_dispatcher,
    ))));
    for (c, se) in &collected.search_engines {
        if config_manager.find_configurable(c.component_id()).is_some() {
            session_dispatcher
                .components()
                .register_search_engine(se.clone());
        } else {
            warn!(
                "组件 {} 的 Configurable 注册失败，跳过 SearchEngine 注册",
                c.component_id()
            );
        }
    }
    for (c, sb) in &collected.score_boosters {
        if config_manager.find_configurable(c.component_id()).is_some() {
            session_dispatcher
                .components()
                .register_score_booster(sb.clone());
        } else {
            warn!(
                "组件 {} 的 Configurable 注册失败，跳过 ScoreBooster 注册",
                c.component_id()
            );
        }
    }
    for (c, ds) in &collected.data_sources {
        if config_manager.find_configurable(c.component_id()).is_some() {
            session_dispatcher
                .components()
                .register_data_source(ds.clone());
        } else {
            warn!(
                "组件 {} 的 Configurable 注册失败，跳过 DataSource 注册",
                c.component_id()
            );
        }
    }
    for (c, ko) in &collected.keyword_optimizers {
        if config_manager.find_configurable(c.component_id()).is_some() {
            session_dispatcher
                .components()
                .register_keyword_optimizer(ko.clone());
        } else {
            warn!(
                "组件 {} 的 Configurable 注册失败，跳过 KeywordOptimizer 注册",
                c.component_id()
            );
        }
    }
    for (c, ki) in &collected.keyword_injectors {
        if config_manager.find_configurable(c.component_id()).is_some() {
            session_dispatcher
                .components()
                .register_keyword_injector(ki.clone());
        } else {
            warn!(
                "组件 {} 的 Configurable 注册失败，跳过 KeywordInjector 注册",
                c.component_id()
            );
        }
    }
    // 内置插件 init 循环已下移至 Phase B（持久化语言同步之后）：
    // init_ctx.locale 必须携带持久化语言（Phase A 时 I18nManager.current 还是系统默认语言）。

    info!(
        "Phase A 完成: 共注册 {} 个组件",
        config_manager.get_all_components().len(),
    );

    // 注册快捷键回调：按下全局快捷键时切换搜索栏显示/隐藏
    info!("正在注册快捷键回调（search_bar_toggle）...");
    let core_handle_for_hotkey = state.get_core_handle();
    let host_api_for_hotkey = host_api.clone();
    let session_router_for_hotkey = session_dispatcher.clone();
    let config_manager_for_hotkey = config_manager.clone();
    let app_handle_for_hotkey = state.get_main_handle();
    core_handle_for_hotkey.register_hotkey_callback(
        "search_bar_toggle",
        HotkeyEventFilter::All,
        Arc::new(move |event| {
            debug!("收到快捷键事件: {:?}", event);
            let host_api = host_api_for_hotkey.clone();
            let session_dispatcher = session_router_for_hotkey.clone();
            let config_manager = config_manager_for_hotkey.clone();
            let app_handle = app_handle_for_hotkey.clone();
            tauri::async_runtime::spawn(async move {
                if host_api.is_window_visible() {
                    save_window_position_if_drag(&config_manager, &app_handle).await;
                    host_api.hide_window().await;
                } else {
                    if !prepare_window_position(&config_manager, &host_api, &app_handle).await {
                        return;
                    }
                    let _ = session_dispatcher.on_search_bar_wake().await;
                    host_api.show_window().await;
                }
            });
        }),
    );

    // ========================================================================
    // Phase B: 加载持久化配置
    // ========================================================================
    info!("=== Phase B: 加载持久化配置 ===");
    if let Err(e) = config_manager.load_from_storage().await {
        warn!("加载持久化配置失败: {}", e);
    }
    sync_backend_theme(&state.get_host_api(), &config_manager);
    // 持久化语言在配置加载后才可知：同步后端翻译服务并重建托盘菜单
    sync_backend_language(state, &config_manager);
    // 内置触发式插件（translator/calculator）在持久化启用状态加载后注册：
    // 按 is_enabled 决定是否建立触发词路由（用户禁用过的插件重启后不路由，
    // 与运行时开关语义一致）。放在 Phase B 之后，is_enabled 才能读到持久化结果。
    for (c, p) in &collected.plugins {
        if config_manager.find_configurable(c.component_id()).is_some() {
            // 必须走 register_plugin_with_triggers（统一入口）：
            // 仅调用 plugin_registry().register 不会建立触发词索引，
            // 会导致内置触发式插件（translator/calculator）路由失效。
            let enabled = config_manager.is_enabled(c.component_id());
            session_dispatcher.register_plugin_with_triggers(p.clone(), enabled);
        } else {
            warn!(
                "组件 {} 的 Configurable 注册失败，跳过 Plugin 注册",
                c.component_id()
            );
        }
    }

    // 内置插件全部注册后统一执行 init：向插件发放绑定身份的 PluginHandle
    // （插件在 init 中保存句柄，供 query/execute_action 访问平台能力）。
    // 此处循环仅覆盖 Phase A 已注册的内置组件；远端插件 init 由
    // SessionDispatcher 在 ConfigEvent::PluginRegistered 处理器经 plugin/init RPC
    // 调用（注册完成后触发，不重复初始化）。
    // 放在持久化语言同步之后：init_ctx.locale 需携带用户持久化语言，
    // 而非 Phase A 时的系统默认语言。
    let trace_id = generate_trace_id();
    let mut init_ctx = PluginContext::new(&trace_id);
    init_ctx.locale = state.get_i18n_manager().current_language();
    for plugin in session_dispatcher.plugin_registry().get_all() {
        let plugin_id = plugin.metadata().id.clone();
        // todo!: 这里是直接使用的默认的权限来注册的。之后可以优化成，让插件支持自己设置需要的权限
        let handle = host_api.register(&plugin_id, PluginSdkConfig::default());
        plugin
            .init(&init_ctx, Some(handle))
            .await
            .expect("内置插件初始化失败");
    }
    info!("内置插件 init 完成（PluginHandle 已发放）");

    info!("构建候选管道...");
    let mut candidate_pipeline = session_dispatcher
        .components()
        .build_candidate_pipeline(&config_manager);

    // 从 BiasConfig 组件加载固定偏移量规则并注入到候选管道
    let rules = config_manager
        .get_settings("bias-config")
        .and_then(|v| serde_json::from_value::<BiasSettings>(v).ok())
        .map(|settings| bias_settings_to_rules(&settings))
        .unwrap_or_default();
    if !rules.is_empty() {
        info!("从持久化配置加载 {} 条偏置偏移量规则", rules.len());
    }
    candidate_pipeline.set_bias_rules(rules);

    info!("正在收集候选项（此时各组件已持有用户持久化配置）...");
    // 插件候选项经统一合并入口并入缓存
    let candidates = session_dispatcher.merge_plugin_candidates(candidate_pipeline.collect().await);
    info!(
        "候选项收集完成，共 {} 个",
        candidates.get_candidates().len()
    );

    info!("根据已注册且启用的搜索引擎与增强器重建搜索管道...");
    session_dispatcher.rebuild_search_pipeline();

    // 内置模型提供方按当前配置构建并聚合模型清单（配置组件已在 Phase A 注册）
    let model_manager = state.get_model_manager();
    model_manager.register_builtin_providers(&config_manager);
    model_manager.refresh_models().await;

    info!("更新 SessionDispatcher 状态...");
    session_dispatcher
        .set_candidate_pipeline(candidate_pipeline)
        .await;
    session_dispatcher.set_cached_candidates(candidates);

    // 收集内置组件 id 集合：plugin-host 冲突预检数据源（第三方组件不得与内置撞 id）。
    // 由调用方（init_app_state）传给 PluginManager::init_host_manager 注入。
    let builtin_component_ids: HashSet<String> = collected
        .configurables()
        .iter()
        .map(|c| c.component_id().to_string())
        .collect();

    info!(
        "插件系统初始化完成，已注册 {} 个组件，缓存 {} 个候选项",
        config_manager.get_all_components().len(),
        session_dispatcher.get_cached_candidates_count()
    );
    builtin_component_ids
}

/// 启动 AppCommand 消费者 task。
///
/// 该 task 从 channel 中接收 BuiltinCommandExecutor / TrayManager 发出的应用级命令，
/// 持有 AppState 访问所有必需的服务（SessionDispatcher、HostApi、ConfigManager 等）。
fn spawn_app_command_consumer(
    mut rx: tokio::sync::mpsc::Receiver<app_command::AppCommand>,
    state: Arc<AppState>,
) {
    tauri::async_runtime::spawn(async move {
        while let Some(cmd) = rx.recv().await {
            debug!("AppCommand 消费者: 收到命令 {:?}", cmd);
            match cmd {
                app_command::AppCommand::ShowSettings => {
                    let app_handle = state.get_main_handle();
                    if let Some(window) = app_handle.get_webview_window("setting_window") {
                        crate::window::show_and_focus_settings_window(&window);
                    }
                }
                app_command::AppCommand::RefreshCandidates => {
                    let session_dispatcher = state.get_session_dispatcher();
                    session_dispatcher.refresh_candidates().await;
                    let count = session_dispatcher.get_cached_candidates_count();
                    info!("AppCommand: 候选项刷新完成，共 {} 个", count);
                }
                app_command::AppCommand::ReregisterHotkeys => {
                    let config_manager = state.get_config_manager();
                    let host_api = state.get_host_api();
                    // 从配置管理器读取快捷键配置并重新注册
                    let hotkey_config = config_manager
                        .get_settings("hotkey-config")
                        .and_then(|v| serde_json::from_value::<HotkeySettings>(v).ok())
                        .map(|settings| settings_to_hotkey_config(&settings));
                    if let Some(config) = hotkey_config {
                        if let Err(e) = host_api.apply_hotkey_config(&config).await {
                            warn!("重新注册快捷键失败: {:?}", e);
                        } else {
                            info!("AppCommand: 快捷键重新注册成功");
                        }
                    } else {
                        warn!("AppCommand: 无法读取快捷键配置 (hotkey-config)");
                    }
                }
                app_command::AppCommand::ToggleGameMode => {
                    let new_state = !state.get_game_mode();
                    state.set_game_mode(new_state);
                    // 更新托盘菜单复选框状态
                    if let Some(tray) = state.get_tray_manager() {
                        tray.set_game_mode_checked(new_state);
                    }

                    let host_api = state.get_host_api();
                    if new_state {
                        // 游戏模式启用：注销所有快捷键，避免游戏中被弹出干扰
                        if let Err(e) = host_api.unregister_all_hotkeys().await {
                            warn!("AppCommand: 游戏模式启用时注销快捷键失败: {:?}", e);
                        } else {
                            info!("AppCommand: 游戏模式已启用，快捷键已注销");
                        }
                    } else {
                        // 游戏模式关闭：从配置读取快捷键并重新注册
                        let config_manager = state.get_config_manager();
                        let hotkey_config = config_manager
                            .get_settings("hotkey-config")
                            .and_then(|v| serde_json::from_value::<HotkeySettings>(v).ok())
                            .map(|settings| settings_to_hotkey_config(&settings));
                        if let Some(config) = hotkey_config {
                            if let Err(e) = host_api.apply_hotkey_config(&config).await {
                                warn!("AppCommand: 游戏模式关闭时重新注册快捷键失败: {:?}", e);
                            } else {
                                info!("AppCommand: 游戏模式已关闭，快捷键已恢复");
                            }
                        } else {
                            warn!("AppCommand: 游戏模式关闭: 无法读取快捷键配置 (hotkey-config)");
                        }
                    }
                }
                app_command::AppCommand::ExitProgram => {
                    info!("AppCommand: 退出程序");
                    let app_handle = state.get_main_handle();
                    app_handle.exit(0);
                }
            }
        }
    });
}

/// 注册系统主题变化监控（宿主侧回调）。
///
/// 平台实现通过 `crate::platform::start_system_theme_monitor` 在编译期注入。
/// 由 setup 在应用启动时调用一次，主题变化时推送前端事件并记录日志。
pub(crate) fn init_system_theme_monitor(app_handle: tauri::AppHandle) {
    use crate::platform::start_system_theme_monitor;
    use zerolaunch_plugin_api::services::Theme;

    if let Err(error) = start_system_theme_monitor(move |theme: Theme| {
        let payload = theme.as_str();
        info!(payload, "系统主题变化");
        let _ = app_handle.emit("system-theme-changed", payload);
    }) {
        warn!(error = %error, "系统主题监控启动失败");
    }
}
