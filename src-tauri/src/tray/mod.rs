use parking_lot::RwLock;
use std::sync::Arc;
use tauri::{
    image::Image,
    menu::{Menu, MenuBuilder, MenuItem},
    tray::{TrayIcon, TrayIconBuilder, TrayIconEvent},
    App, AppHandle, Manager, Runtime,
};
use tracing::{debug, error, info, warn};

use crate::sdk::HostApi;

const MENU_ID_SHOW_SETTINGS: &str = "show_setting_window";
const MENU_ID_EXIT_PROGRAM: &str = "exit_program";

enum MenuEventId {
    ShowSettingWindow,
    ExitProgram,
    Unknown(String),
}

impl From<&str> for MenuEventId {
    fn from(id: &str) -> Self {
        match id {
            MENU_ID_SHOW_SETTINGS => MenuEventId::ShowSettingWindow,
            MENU_ID_EXIT_PROGRAM => MenuEventId::ExitProgram,
            _ => MenuEventId::Unknown(id.to_string()),
        }
    }
}

/// 系统托盘管理器。
///
/// 负责托盘图标、右键菜单和事件处理的全生命周期。
/// 采用 Inner 模式：外层 TrayManager 通过 RwLock 委托所有操作给 Inner。
pub struct TrayManager {
    inner: RwLock<TrayManagerInner>,
}

struct TrayManagerInner {
    tray_icon: Option<TrayIcon>,
    menu: Option<Menu<tauri::Wry>>,
    host_api: Arc<HostApi>,
    app_handle: Option<AppHandle>,
}

impl TrayManager {
    /// 创建 TrayManager 实例。
    ///
    /// 参数：host_api - 用于解析内置图标路径。
    pub fn new(host_api: Arc<HostApi>) -> Self {
        Self {
            inner: RwLock::new(TrayManagerInner {
                tray_icon: None,
                menu: None,
                host_api,
                app_handle: None,
            }),
        }
    }

    /// 初始化系统托盘（注册事件、创建图标，含失败重试）。
    ///
    /// 参数：app - Tauri App 实例，用于注册托盘双击事件。
    pub async fn init(&self, app: &mut App) {
        let app_handle = app.handle().clone();

        // 注册双击事件：切换主窗口显示/隐藏
        app.on_tray_icon_event(move |tray_app_handle, event| {
            if let TrayIconEvent::DoubleClick { .. } = event {
                if let Some(window) = tray_app_handle.get_webview_window("main") {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                    } else {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
        });

        // 存储应用句柄
        {
            let mut inner = self.inner.write();
            inner.app_handle = Some(app_handle.clone());
        }

        // 首次尝试创建托盘
        if self.try_create_and_set_tray(&app_handle).is_ok() {
            debug!("System tray initialized successfully on first attempt.");
            return;
        }

        // 重试逻辑
        let retry_delays = [1, 2, 2, 3, 5];
        for &delay in &retry_delays {
            warn!(
                "Tray icon creation failed. Retrying in {} seconds...",
                delay
            );
            tokio::time::sleep(std::time::Duration::from_secs(delay)).await;

            if self.try_create_and_set_tray(&app_handle).is_ok() {
                info!("System tray initialized successfully after retry.");
                return;
            }
        }

        error!("Failed to initialize system tray after all retries.");
    }

    /// 根据当前系统主题更新托盘图标。
    pub fn update_icon_theme(&self) {
        let inner = self.inner.read();
        let app_handle = match &inner.app_handle {
            Some(h) => h,
            None => {
                warn!("TrayManager not initialized, skipping icon theme update");
                return;
            }
        };

        let use_white_icon = should_use_white_tray_icon(app_handle);
        let icon_key = if use_white_icon {
            "tray_icon_white"
        } else {
            "tray_icon"
        };

        if let Some(path) = inner.host_api.get_app_icon_path(icon_key) {
            match Image::from_path(&path) {
                Ok(icon) => {
                    if let Some(ref tray_icon) = inner.tray_icon {
                        if let Err(e) = tray_icon.set_icon(Some(icon)) {
                            warn!("Failed to update tray icon: {:?}", e);
                        } else {
                            debug!("Tray icon updated to {}", icon_key);
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to load tray icon from path {:?}: {:?}", path, e);
                }
            }
        } else {
            warn!(
                "Tray icon key '{}' not found in app resource service",
                icon_key
            );
        }
    }

    /// 重建托盘菜单（用于语言切换等场景）。
    pub fn update_menu_language(&self) {
        let inner = self.inner.read();
        let app_handle = match &inner.app_handle {
            Some(h) => h,
            None => {
                warn!("TrayManager not initialized, skipping menu language update");
                return;
            }
        };

        let menu = match build_tray_menu(app_handle) {
            Ok(m) => m,
            Err(e) => {
                warn!("Failed to rebuild tray menu: {:?}", e);
                return;
            }
        };

        if let Some(ref tray_icon) = inner.tray_icon {
            if let Err(e) = tray_icon.set_menu(Some(menu.clone())) {
                warn!("Failed to update tray menu: {:?}", e);
            }
            if let Err(e) = tray_icon.set_tooltip(Some("zerolaunch-rs")) {
                warn!("Failed to update tray tooltip: {:?}", e);
            }
        }
        drop(inner);

        let mut inner = self.inner.write();
        inner.menu = Some(menu);
        debug!("Tray menu language updated.");
    }

    /// 显示设置窗口。
    pub fn show_settings_window(&self) {
        let inner = self.inner.read();
        let app_handle = match &inner.app_handle {
            Some(h) => h,
            None => {
                warn!("TrayManager not initialized");
                return;
            }
        };

        if let Some(window) = app_handle.get_webview_window("setting_window") {
            if let Err(e) = window.show() {
                warn!("Failed to show setting window: {:?}", e);
            } else {
                let _ = window.set_focus();
            }
        }
    }

    /// 退出程序。
    pub fn exit_program(&self) {
        let inner = self.inner.read();
        if let Some(ref app_handle) = inner.app_handle {
            app_handle.exit(0);
        }
    }

    // ===== 内部方法 =====

    /// 尝试创建托盘图标并保存到 Inner。
    /// Inner 层方法，由外壳委托调用。
    fn try_create_and_set_tray(&self, app_handle: &AppHandle) -> Result<(), ()> {
        let menu = build_tray_menu(app_handle).map_err(|e| {
            warn!("Failed to build tray menu: {:?}", e);
        })?;

        let tray_icon = self
            .create_tray_icon(app_handle, menu.clone())
            .map_err(|e| {
                warn!("Failed to create tray icon: {:?}", e);
            })?;

        let mut inner = self.inner.write();
        inner.tray_icon = Some(tray_icon);
        inner.menu = Some(menu);

        Ok(())
    }

    /// 创建托盘图标实例。
    /// Inner 层方法，由外壳委托调用。
    fn create_tray_icon(
        &self,
        app_handle: &AppHandle,
        menu: Menu<tauri::Wry>,
    ) -> tauri::Result<TrayIcon> {
        let use_white_icon = should_use_white_tray_icon(app_handle);
        let icon_key = if use_white_icon {
            "tray_icon_white"
        } else {
            "tray_icon"
        };

        let inner = self.inner.read();
        let icon_path = inner
            .host_api
            .get_app_icon_path(icon_key)
            .unwrap_or_else(|| panic!("Tray icon path '{}' not found in app resources", icon_key));
        drop(inner);

        let icon = Image::from_path(&icon_path)
            .unwrap_or_else(|_| panic!("Failed to load tray icon from path {:?}", icon_path));

        TrayIconBuilder::new()
            .menu(&menu)
            .icon(icon)
            .tooltip("zerolaunch-rs")
            .show_menu_on_left_click(false)
            .on_menu_event(move |app, event| {
                let event_id = MenuEventId::from(event.id().as_ref());
                match event_id {
                    MenuEventId::ShowSettingWindow => show_setting_window_inner(app),
                    MenuEventId::ExitProgram => {
                        let app_clone = app.clone();
                        tauri::async_runtime::spawn(async move {
                            app_clone.exit(0);
                        });
                    }
                    MenuEventId::Unknown(id) => {
                        warn!("Unknown tray menu event: {}", id);
                    }
                }
                debug!("Menu ID: {}", event.id().0);
            })
            .build(app_handle)
    }
}

// ===== 自由函数（供闭包使用，不捕获 TrayManager） =====

fn build_tray_menu<R: Runtime>(app_handle: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let show_settings = MenuItem::with_id(
        app_handle,
        MENU_ID_SHOW_SETTINGS,
        "设置窗口",
        true,
        None::<&str>,
    )?;
    let exit_program = MenuItem::with_id(
        app_handle,
        MENU_ID_EXIT_PROGRAM,
        "退出程序",
        true,
        None::<&str>,
    )?;

    MenuBuilder::new(app_handle)
        .item(&show_settings)
        .separator()
        .item(&exit_program)
        .build()
}

fn should_use_white_tray_icon(app_handle: &AppHandle) -> bool {
    if let Some(window) = app_handle.get_webview_window("main") {
        match window.theme() {
            Ok(theme) => theme == tauri::Theme::Dark,
            Err(_) => false,
        }
    } else {
        false
    }
}

/// 托盘菜单回调：显示设置窗口（不捕获 TrayManager，供闭包直接使用）。
fn show_setting_window_inner(app_handle: &AppHandle) {
    if let Some(window) = app_handle.get_webview_window("setting_window") {
        if let Err(e) = window.show() {
            warn!("Failed to show setting window: {:?}", e);
        } else {
            let _ = window.set_focus();
        }
    }
}
