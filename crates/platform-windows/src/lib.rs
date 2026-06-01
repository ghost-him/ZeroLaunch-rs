#![cfg(target_os = "windows")]
//! Windows platform implementations of zerolaunch-plugin-api traits.

mod app_enumerator;
mod app_launcher;
mod autostart;
mod capabilities;
mod com_guard;
mod focus_monitor;
mod hotkey;
mod icon;
mod installation_monitor;
mod lnk_resolver;
mod parameter_providers;
mod path_resolver;
mod resource_loader;
mod shell;
mod utils;
mod window;
mod window_positioner;

pub use app_enumerator::WindowsAppEnumerator;
pub use app_launcher::WindowsAppLauncher;
pub use autostart::WindowsAutoStartManager;
pub use capabilities::windows_capabilities;
pub use com_guard::ComGuard;
pub use focus_monitor::WindowsFocusMonitor;
pub use hotkey::WindowsHotkeyManager;
pub use icon::WindowsIconExtractor;
pub use installation_monitor::WindowsInstallationMonitor;
pub use lnk_resolver::WindowsLnkResolver;
pub use parameter_providers::{
    WindowsClipboardProvider, WindowsSelectionProvider, WindowsWindowHandleProvider,
};
pub use path_resolver::WindowsPathResolver;
pub use resource_loader::WindowsResourceLoader;
pub use shell::WindowsShellExecutor;
pub use window::WindowsWindowManager;
pub use window_positioner::WindowsWindowPositioner;
