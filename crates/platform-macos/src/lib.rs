#![cfg(target_os = "macos")]
//! macOS implementations for the platform-neutral Plugin API contracts.

mod app_enumerator;
mod app_launcher;
mod autostart;
mod capabilities;
mod clipboard;
mod focus_monitor;
mod fullscreen;
mod hotkey;
mod icon;
mod installation_monitor;
mod lnk_resolver;
mod parameter_providers;
mod path_resolver;
mod resource_loader;
mod shell;
mod theme;
mod utils;
mod window;
mod window_positioner;

pub use app_enumerator::MacosAppEnumerator;
pub use app_launcher::MacosAppLauncher;
pub use autostart::MacosAutoStartManager;
pub use capabilities::macos_capabilities;
pub use clipboard::MacosClipboardManager;
pub use focus_monitor::MacosFocusMonitor;
pub use fullscreen::is_foreground_fullscreen;
pub use hotkey::MacosHotkeyManager;
pub use icon::MacosIconExtractor;
pub use installation_monitor::MacosInstallationMonitor;
pub use lnk_resolver::MacosLnkResolver;
pub use parameter_providers::{
    MacosClipboardProvider, MacosSelectionProvider, MacosWindowHandleProvider,
};
pub use path_resolver::MacosPathResolver;
pub use resource_loader::MacosResourceLoader;
pub use shell::MacosShellExecutor;
pub use theme::{start_system_theme_monitor, MacosThemeProvider};
pub use utils::{init_com, os_version};
pub use window::MacosWindowManager;
pub use window_positioner::MacosWindowPositioner;
