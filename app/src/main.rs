//! ZeroLaunch GPUI 搜索窗入口。
//!
//! 复用 `zerolaunch-rs` 的内核装配，但窗口与输入链路完全走 GPUI：
//! 启动时安装 `GpuiHost`，初始化内核，再打开无边框搜索窗口。

mod deeplink;
mod host;
mod platform;
mod search;
mod settings;
mod shell;
mod tray;

use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::{
    point, px, size, App, AppContext as _, Bounds, TitlebarOptions, WindowBackgroundAppearance,
    WindowBounds, WindowDecorations, WindowKind, WindowOptions,
};
use zerolaunch_platform_windows::{
    acquire_single_instance, exe_dir, windows_platform_services, ComGuard, SingleInstanceStatus,
    WindowsPathResolver,
};
use zerolaunch_plugin_api::services::path::path_resolver::PathResolver;
use zerolaunch_plugin_api::services::path::KnownPath;
use zerolaunch_plugin_api::services::AppResourceService;
use zerolaunch_rs_lib::core::config::ConfigManager;
use zerolaunch_rs_lib::core::host::Host as _;
use zerolaunch_rs_lib::kernel::{self, KernelOptions, KernelPaths};
use zerolaunch_rs_lib::logging::{init_logging, log_application_start};
use zerolaunch_rs_lib::runtime;
use zerolaunch_rs_lib::state::app_state::AppState;

use crate::host::{GpuiHost, HostCommand};
use crate::search::SearchView;
use crate::settings::SettingsView;
use crate::shell::PluginUiHost;

const APP_ID: &str = "com.zerolaunch-rs.app";

fn main() {
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(|cx| {
            gpui_kit::init(cx);

            let path_resolver = Arc::new(WindowsPathResolver::new());
            let log_dir = path_resolver
                .resolve_path(KnownPath::AppLogDir)
                .expect("无法解析日志目录");
            let _log_guard = init_logging(&log_dir, None);
            log_application_start();

            let _com_guard = unsafe { ComGuard::init() };
            std::mem::forget(_com_guard);

            let state = Arc::new(AppState::new());
            let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel();
            let host = Arc::new(GpuiHost::new(command_tx));
            state.set_host(host.clone());

            let host_for_instance = host.clone();
            let state_for_instance = state.clone();
            let status = acquire_single_instance(
                APP_ID,
                Arc::new(move |args, _cwd| {
                    host_for_instance.show_main_window();
                    deeplink::dispatch(&state_for_instance, &args);
                }),
            );
            if status == SingleInstanceStatus::Secondary {
                std::process::exit(0);
            }

            if let Ok(exe) = std::env::current_exe() {
                if let Err(error) = deeplink::register_protocol(&exe) {
                    tracing::warn!("注册深链接协议失败: {error}");
                }
            }

            init_kernel(&state, path_resolver);
            host.set_i18n(state.get_i18n_manager());
            let tray_icon = exe_dir()
                .map(|dir| dir.join("icons").join("icon.ico"))
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            host.init_tray(tray_icon, state.is_show_tray_icon(), state.get_game_mode());
            deeplink::dispatch(&state, &std::env::args().collect::<Vec<_>>());
            let plugin_ui = match PluginUiHost::new(state.clone(), host.clone(), cx) {
                Ok(host) => Some(Rc::new(host)),
                Err(error) => {
                    tracing::error!("初始化插件脚本宿主失败: {error}");
                    None
                }
            };
            open_search_window(
                cx,
                state.clone(),
                host.clone(),
                command_rx,
                plugin_ui.clone(),
            );
            open_settings_window(cx, state, host, plugin_ui);
        });
}

/// 初始化内核（配置、插件、CLI 与热键）。
/// 参数：state - 已安装 Host 的应用状态；path_resolver - 平台路径解析器。
/// 返回：无。
fn init_kernel(state: &Arc<AppState>, path_resolver: Arc<WindowsPathResolver>) {
    let app_data_dir = path_resolver
        .resolve_path(KnownPath::AppDataDir)
        .expect("无法解析应用数据目录");
    let icon_cache_dir = path_resolver
        .resolve_path(KnownPath::AppIconCacheDir)
        .expect("无法解析图标缓存目录");
    let config_dir = path_resolver
        .resolve_path(KnownPath::AppConfigDir)
        .expect("无法解析配置目录");
    let resource_dir = exe_dir().expect("无法获取资源目录");
    let app_resource =
        AppResourceService::new(resource_dir.join("icons").to_string_lossy().to_string());
    let default_app_icon = app_resource
        .get_icon_path("tips")
        .unwrap_or_else(|| ".".into());
    let default_web_icon = app_resource
        .get_icon_path("web_pages")
        .unwrap_or_else(|| ".".into());

    // 焦点监控由 GPUI 激活观察者承担，平台工厂的主窗口句柄传 0；
    // 其余平台能力不依赖该句柄。
    let platform = windows_platform_services(0, path_resolver, default_app_icon, default_web_icon);

    runtime::block_on(kernel::init(
        state,
        platform,
        KernelPaths {
            app_data_dir,
            icon_cache_dir,
            config_dir,
            resource_dir,
        },
        KernelOptions::default(),
    ));
}

/// 打开透明无边框搜索窗口并绑定宿主句柄。
/// 参数：cx - GPUI App；state - 已初始化内核；host - 宿主实现。
/// 返回：无。
fn open_search_window(
    cx: &mut App,
    state: Arc<AppState>,
    host: Arc<GpuiHost>,
    commands: tokio::sync::mpsc::UnboundedReceiver<HostCommand>,
    plugin_ui: Option<Rc<PluginUiHost>>,
) {
    let config = state.get_config_manager();
    let width = metric(&config, "appearance-config", "window_width", 800.0) as f32;
    let height = metric(&config, "appearance-config", "search_bar_height", 72.0) as f32;

    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: size(px(width), px(height)),
        })),
        titlebar: Some(TitlebarOptions {
            title: None,
            appears_transparent: true,
            traffic_light_position: None,
        }),
        focus: true,
        show: false,
        kind: WindowKind::PopUp,
        is_movable: true,
        is_resizable: false,
        is_minimizable: false,
        window_background: WindowBackgroundAppearance::Transparent,
        window_decorations: Some(WindowDecorations::Client),
        ..Default::default()
    };

    let state_for_view = state.clone();
    let host_for_view = host.clone();
    let plugin_ui_for_view = plugin_ui.clone();
    let opened = gpui_kit::open_window(options, cx, move |window, cx| {
        cx.new(|cx| {
            SearchView::new(
                state_for_view,
                host_for_view,
                commands,
                plugin_ui_for_view,
                window,
                cx,
            )
        })
    });

    let Ok((handle, _view)) = opened else {
        tracing::error!("打开 GPUI 搜索窗口失败");
        return;
    };

    let hwnd = handle
        .update(cx, |_, window, _| platform::window_hwnd(window))
        .ok()
        .flatten()
        .unwrap_or(0);
    host.attach(hwnd, true);
    // 启动时与旧版一致保持隐藏，由全局热键唤出。
    host.hide_main_window();
}

/// 打开设置窗口并绑定宿主句柄。
/// 参数：cx - GPUI App；state - 已初始化内核；host - 宿主实现。
/// 返回：无；窗口初始隐藏，由 Host 或搜索窗快捷键唤出。
fn open_settings_window(
    cx: &mut App,
    state: Arc<AppState>,
    host: Arc<GpuiHost>,
    plugin_ui: Option<Rc<PluginUiHost>>,
) {
    let i18n = state.get_i18n_manager();
    let settings_title = i18n.t(&i18n.current_language(), "settings.title");
    host.set_settings_window_title(&settings_title);
    let monitor = host
        .main_window_monitor()
        .or_else(platform::cursor_monitor_info)
        .or_else(platform::primary_monitor_info);
    let scale = monitor
        .as_ref()
        .map(|monitor| monitor.scale_factor as f32)
        .unwrap_or(1.0)
        .max(1.0);
    let logical_width = monitor
        .as_ref()
        .map(|monitor| (monitor.width as f32 / scale - 160.0).min(900.0))
        .unwrap_or(900.0)
        .max(640.0);
    let logical_height = monitor
        .as_ref()
        .map(|monitor| (monitor.height as f32 / scale - 160.0).min(620.0))
        .unwrap_or(620.0)
        .max(420.0);
    let (window_x, window_y) = (80, 60);
    let physical_width = (logical_width * scale + 16.0 * scale).round() as i32;
    let physical_height = (logical_height * scale + 39.0 * scale).round() as i32;
    let origin = point(px(window_x as f32), px(window_y as f32));
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin,
            size: size(px(logical_width), px(logical_height)),
        })),
        titlebar: Some(TitlebarOptions {
            title: None,
            appears_transparent: false,
            traffic_light_position: None,
        }),
        focus: false,
        show: false,
        kind: WindowKind::Normal,
        is_movable: true,
        is_resizable: true,
        is_minimizable: true,
        window_decorations: Some(WindowDecorations::Server),
        ..Default::default()
    };

    let state_for_view = state;
    let host_for_view = host.clone();
    let plugin_ui_for_view = plugin_ui;
    host.set_settings_bounds((window_x, window_y, physical_width, physical_height));
    let opened = gpui_kit::open_window(options, cx, move |window, cx| {
        cx.new(|cx| {
            SettingsView::new(
                state_for_view,
                host_for_view,
                plugin_ui_for_view,
                window,
                cx,
            )
        })
    });

    let Ok((handle, _view)) = opened else {
        tracing::error!("打开 GPUI 设置窗口失败");
        return;
    };
    let hwnd = handle
        .update(cx, |_, window, _| platform::window_hwnd(window))
        .ok()
        .flatten()
        .unwrap_or(0);
    host.attach_settings(hwnd);
    host.hide_settings_window();
}

/// 读取配置中的数值项。
/// 参数：config - 配置管理器；component/key - 组件与字段；fallback - 缺失或非法时的默认值。
/// 返回：有限的 f64 取值。
fn metric(config: &ConfigManager, component: &str, key: &str, fallback: f64) -> f64 {
    config
        .get_component_setting(component, key)
        .and_then(|value| value.as_f64())
        .filter(|value| value.is_finite())
        .unwrap_or(fallback)
}
