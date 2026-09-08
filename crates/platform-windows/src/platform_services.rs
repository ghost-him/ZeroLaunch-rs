//! windows_platform_services — Windows 平台实现的统一工厂。
//!
//! 组装权归属平台 crate：宿主（src-tauri）只按端口整体注入 PlatformServices，
//! 无需逐个 import 具体 Windows 实现类型。宿主级服务（storage/model/timer 等）
//! 不在本工厂内，由宿主自行装配。

use std::sync::Arc;

use tauri::AppHandle;
use zerolaunch_plugin_api::platform::PlatformServices;

use crate::app_enumerator::WindowsAppEnumerator;
use crate::app_launcher::WindowsAppLauncher;
use crate::autostart::WindowsAutoStartManager;
use crate::capabilities::windows_capabilities;
use crate::clipboard::WindowsClipboardManager;
use crate::focus_monitor::WindowsFocusMonitor;
use crate::hotkey::WindowsHotkeyManager;
use crate::icon::WindowsIconExtractor;
use crate::installation_monitor::WindowsInstallationMonitor;
use crate::lnk_resolver::WindowsLnkResolver;
use crate::parameter_providers::{
    WindowsClipboardProvider, WindowsSelectionProvider, WindowsWindowHandleProvider,
};
use crate::path_resolver::WindowsPathResolver;
use crate::resource_loader::WindowsResourceLoader;
use crate::shell::WindowsShellExecutor;
use crate::theme::WindowsThemeProvider;
use crate::window::WindowsWindowManager;
use crate::window_positioner::WindowsWindowPositioner;

/// 装配 Windows 平台实现的统一集合。
/// 参数：app_handle - Tauri AppHandle（热键/焦点监控等平台管理器依赖）；
///       path_resolver - 路径解析器（宿主启动早期已构造，用于日志目录解析）；
///       default_app_icon_path / default_web_icon_path - 图标提取默认回退路径。
/// 返回：PlatformServices（含能力集与全部平台服务 trait 对象）。
pub fn windows_platform_services(
    app_handle: Arc<AppHandle>,
    path_resolver: Arc<WindowsPathResolver>,
    default_app_icon_path: String,
    default_web_icon_path: String,
) -> PlatformServices {
    PlatformServices {
        capabilities: windows_capabilities(),
        icon_extractor: Arc::new(WindowsIconExtractor::new(
            default_app_icon_path,
            default_web_icon_path,
        )),
        shell_executor: Arc::new(WindowsShellExecutor::new()),
        window_manager: Arc::new(WindowsWindowManager::new()),
        window_positioner: Arc::new(WindowsWindowPositioner::new()),
        path_resolver,
        app_enumerator: Arc::new(WindowsAppEnumerator::new()),
        app_launcher: Arc::new(WindowsAppLauncher::new()),
        lnk_resolver: Arc::new(WindowsLnkResolver::new()),
        resource_loader: Arc::new(WindowsResourceLoader::new()),
        clipboard_provider: Arc::new(WindowsClipboardProvider),
        window_handle_provider: Arc::new(WindowsWindowHandleProvider),
        selection_provider: Arc::new(WindowsSelectionProvider),
        autostart_manager: Arc::new(WindowsAutoStartManager::new()),
        hotkey_manager: Arc::new(WindowsHotkeyManager::new(app_handle.clone())),
        installation_monitor: Arc::new(WindowsInstallationMonitor::new()),
        focus_monitor: Arc::new(WindowsFocusMonitor::new(app_handle)),
        clipboard_manager: Arc::new(WindowsClipboardManager::new()),
        theme_provider: Arc::new(WindowsThemeProvider),
    }
}
