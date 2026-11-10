//! Tauri 外壳适配层。
//!
//! 只保留 Tauri 专属装配：安装 `TauriHost`、主窗口句柄、平台端口与托盘。
//! 业务装配全部在 `kernel.rs`，不依赖 Tauri 类型。

use std::sync::Arc;
use tauri::{App, Manager};
use zerolaunch_platform_windows::{exe_dir, windows_platform_services, WindowsPathResolver};
use zerolaunch_plugin_api::services::AppResourceService;

use crate::kernel::{self, KernelPaths};
use crate::state::app_state::AppState;
use crate::state::tauri_host::TauriHost;

pub(crate) use crate::kernel::{
    do_cleanup_before_exit, init_system_theme_monitor, setting_window_title,
};

/// 初始化内核并创建 Tauri 托盘。
///
/// 参数：`app` — Tauri 应用；`path_resolver` — 平台路径解析器；
/// 其余目录由调用方提前解析（应用数据、图标缓存、配置）。
pub(crate) async fn init_app_state(
    app: &mut App,
    path_resolver: Arc<WindowsPathResolver>,
    app_data_dir: String,
    icon_cache_dir: String,
    config_dir: String,
) {
    let state = app.state::<Arc<AppState>>().inner().clone();

    // 安装 Tauri 宿主实现；后续内核窗口/托盘/事件操作只经 Host 端口。
    let host = Arc::new(TauriHost::new(app.handle().clone()));
    state.set_host(host.clone());

    // 平台端口：Windows 平台实现的统一工厂（组装权在 platform-windows）。
    let main_window_handle = app
        .get_webview_window("main")
        .and_then(|window| window.hwnd().ok())
        .map(|handle| handle.0 as isize)
        .unwrap_or(0);
    let resource_dir = exe_dir().expect("无法获取资源目录");
    let app_resource =
        AppResourceService::new(resource_dir.join("icons").to_string_lossy().to_string());
    let default_app_icon_path = app_resource
        .get_icon_path("tips")
        .unwrap_or_else(|| ".".to_string());
    let default_web_icon_path = app_resource
        .get_icon_path("web_pages")
        .unwrap_or_else(|| ".".to_string());
    let platform = windows_platform_services(
        main_window_handle,
        path_resolver,
        default_app_icon_path,
        default_web_icon_path,
    );

    kernel::init(
        &state,
        platform,
        KernelPaths {
            app_data_dir,
            icon_cache_dir,
            config_dir,
            resource_dir,
        },
        kernel::KernelOptions::default(),
    )
    .await;

    // 托盘依赖内核产出的 HostApi 与 I18nManager，必须在内核初始化后创建。
    let show_tray_icon = state.is_show_tray_icon();
    host.init_tray(
        app,
        state.get_host_api(),
        state.get_i18n_manager(),
        show_tray_icon,
    )
    .await;
}
